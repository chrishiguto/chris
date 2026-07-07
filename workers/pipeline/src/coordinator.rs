//! The publish coordinator: one Durable Object owns every content mutation.
//! Triggers only mark it dirty and schedule the alarm; the alarm runs one
//! serialized reconcile-to-HEAD at a time, so triggers never interleave writes.

use std::collections::BTreeSet;

use content::{
    index_key_at, post_key_at, snapshot_key_sha, source_path, CurrentPointer, Diagnostic,
    IndexEntry, CURRENT_KEY, SNAPSHOT_KEY_SPACE,
};
use futures_util::future::join_all;
use futures_util::StreamExt;
use publish::{CarriedPost, ParsedPost, PostSource};
use worker::{
    console_error, console_log, durable_object, kv::KvStore, DurableObject, Env, Method, Request,
    Response, Result, State,
};

use crate::{
    contents_url, head_ref_url, net, parse_head_ref, parse_tree_listing, reconcile_description,
    tree_url, ReconcileConfig,
};

/// Every caller addresses this one instance, which serializes all publishes.
pub(crate) const COORDINATOR_BINDING: &str = "COORDINATOR";
pub(crate) const COORDINATOR_NAME: &str = "publish";

/// Same namespace the site worker reads.
const KV_BINDING: &str = "BLOG";

/// DO storage keys; `dirty` coalesces triggers that land mid-reconcile.
const CONFIG_KEY: &str = "config";
const DIRTY_KEY: &str = "dirty";
const HISTORY_KEY: &str = "history";
const LAST_STATUS_KEY: &str = "last-status";

/// Snapshots kept for rollback; older ones are swept after a flip.
const KEEP_SNAPSHOTS: usize = 10;
/// How long until the alarm re-fires on its own, healing missed triggers.
const BACKSTOP_MS: i64 = 6 * 60 * 60 * 1000;
const FETCH_CONCURRENCY: usize = 8;

#[durable_object]
pub struct PublishCoordinator {
    state: State,
    env: Env,
}

impl DurableObject for PublishCoordinator {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    /// `POST /trigger`: persist the target, mark dirty, pull the alarm to
    /// now. Triggers during a running reconcile coalesce into one follow-up.
    async fn fetch(&self, mut req: Request) -> Result<Response> {
        if req.method() != Method::Post || req.path() != "/trigger" {
            return Response::error("not found", 404);
        }
        let config: ReconcileConfig = req.json().await?;
        let storage = self.state.storage();
        storage.put(CONFIG_KEY, &config).await?;
        storage.put(DIRTY_KEY, true).await?;
        storage.set_alarm(0_i64).await?;
        Response::ok("reconcile scheduled")
    }

    /// The serialization point: alarms never overlap and auto-retry on
    /// failure, so reconciles run one at a time.
    async fn alarm(&self) -> Result<Response> {
        let storage = self.state.storage();
        // Re-arm the backstop first so a failed run still comes back.
        storage.set_alarm(BACKSTOP_MS).await?;
        storage.put(DIRTY_KEY, false).await?;
        let Some(config) = storage.get::<ReconcileConfig>(CONFIG_KEY).await? else {
            // Fresh namespace: nothing to converge to yet.
            storage.delete_alarm().await?;
            return Response::ok("no reconcile target configured");
        };
        if let Err(err) = self.reconcile(&config).await {
            console_error!("reconcile of {} failed: {err}", config.repository);
            return Err(worker::Error::RustError(err));
        }
        // A trigger landed mid-reconcile: run again now (alarm-overwrite
        // ordering during a running handler isn't guaranteed).
        if storage.get::<bool>(DIRTY_KEY).await?.unwrap_or(false) {
            storage.set_alarm(0_i64).await?;
        }
        Response::ok("reconciled")
    }
}

impl PublishCoordinator {
    /// One convergence pass: observe HEAD, rebuild, flip, retain, purge,
    /// report. Infra errors propagate (the alarm retries); validation
    /// failures carry forward.
    async fn reconcile(&self, config: &ReconcileConfig) -> std::result::Result<(), String> {
        let env = &self.env;
        let repo = &config.repository;

        let kv = env.kv(KV_BINDING).map_err(|err| err.to_string())?;
        let (at_head, previous) = futures_util::join!(
            async {
                let head = head_sha(env, repo, &config.branch).await?;
                let slugs = tree_post_slugs_at(env, repo, &head).await?;
                Ok::<_, String>((head, slugs))
            },
            async {
                let prev_sha = read_pointer(&kv).await?;
                let prev_index = read_index(&kv, prev_sha.as_deref()).await?;
                Ok::<_, String>((prev_sha, prev_index))
            }
        );
        let (head, slugs) = at_head?;
        let (prev_sha, prev_index) = previous?;

        // The tree at `head` listed every path, so a 404 is transport
        // trouble, not a removal.
        let sources: Vec<PostSource> = futures_util::stream::iter(slugs.iter().map(|slug| {
            let head = &head;
            async move {
                let source = net::fetch_content(env, &contents_url(repo, slug, head))
                    .await?
                    .ok_or_else(|| format!("contents fetch for {slug} at {head} returned 404"))?;
                Ok::<_, String>(PostSource {
                    slug: slug.clone(),
                    file: source_path(slug),
                    source,
                })
            }
        }))
        .buffered(FETCH_CONCURRENCY)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<std::result::Result<_, _>>()?;

        let manifest = app::manifest();
        let mut parsed: Vec<ParsedPost> = Vec::new();
        let mut diags: Vec<Diagnostic> = Vec::new();
        let mut failed_slugs: Vec<&str> = Vec::new();
        for (source, result) in sources.iter().zip(publish::check_each(&sources, &manifest)) {
            match result {
                Ok(post) => parsed.push(post),
                Err(errs) => {
                    diags.extend(errs);
                    failed_slugs.push(&source.slug);
                }
            }
        }
        let failed = failed_slugs.len();
        let mut carried: Vec<CarriedPost> = Vec::new();
        for slug in failed_slugs {
            if let Some(entry) = prev_index.iter().find(|entry| entry.slug == slug) {
                let key = post_key_at(prev_sha.as_deref(), slug);
                match kv.get(&key).text().await.map_err(|err| err.to_string())? {
                    Some(payload) => carried.push(CarriedPost {
                        entry: entry.clone(),
                        payload,
                    }),
                    // The previous index named it, so a missing payload is
                    // infra trouble; the post drops from the index, loudly.
                    None => console_error!(
                        "carry-forward of {slug} lost: {key} missing — the post drops from the index"
                    ),
                }
            }
        }
        let carried_count = carried.len();

        let plan = publish::snapshot(&prev_index, &parsed, carried, &head)
            .map_err(|err| err.to_string())?;
        // Post payloads land concurrently; the index goes last so a torn
        // write never leaves an index naming missing posts.
        join_all(plan.post_writes.iter().map(|write| kv_put(&kv, write)))
            .await
            .into_iter()
            .collect::<std::result::Result<(), _>>()?;
        kv_put(&kv, &plan.index_write).await?;
        // Every snapshot key is in place — the flip is the publish.
        let pointer = serde_json::to_string(&CurrentPointer { sha: head.clone() })
            .map_err(|err| err.to_string())?;
        kv.put(CURRENT_KEY, pointer)
            .map_err(|err| err.to_string())?
            .execute()
            .await
            .map_err(|err| err.to_string())?;

        // Retention and purge are best-effort; the publish already happened.
        self.retain(&kv, &head).await;
        net::purge(env, &plan).await;
        self.report(repo, &head, parsed.len(), failed, carried_count, &diags)
            .await;
        console_log!(
            "reconciled {repo}@{head}: {} published, {failed} failed",
            parsed.len()
        );
        Ok(())
    }

    /// Posts the reconcile's status on the HEAD it converged to, skipping
    /// duplicates of the last one that landed.
    async fn report(
        &self,
        repo: &str,
        head: &str,
        published: usize,
        failed: usize,
        carried: usize,
        diags: &[Diagnostic],
    ) {
        let (state, description) = reconcile_description(published, failed, carried, diags);
        let stamp = format!("{head}|{state:?}|{description}");
        let storage = self.state.storage();
        let last: Option<String> = storage.get(LAST_STATUS_KEY).await.unwrap_or_else(|err| {
            console_error!("last-status read failed (a duplicate status may repost): {err}");
            None
        });
        if last.as_deref() == Some(stamp.as_str()) {
            return;
        }
        // Stamp only what landed so a lost status is retried next reconcile.
        if !net::post_status(&self.env, repo, head, state, &description).await {
            return;
        }
        if let Err(err) = storage.put(LAST_STATUS_KEY, &stamp).await {
            console_error!("recording last status failed: {err}");
        }
    }

    /// Sweeps snapshot keys outside the history and whatever `current`
    /// points at now (protects a concurrent break-glass flip). Best-effort.
    async fn retain(&self, kv: &KvStore, head: &str) {
        let storage = self.state.storage();
        let mut history: Vec<String> = storage
            .get(HISTORY_KEY)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        history.retain(|sha| sha != head);
        history.insert(0, head.to_string());
        if history.len() > KEEP_SNAPSHOTS {
            history.truncate(KEEP_SNAPSHOTS);
        }
        if let Err(err) = storage.put(HISTORY_KEY, &history).await {
            console_error!("recording snapshot history failed: {err}");
            return;
        }

        let mut keep: BTreeSet<String> = history.into_iter().collect();
        match read_pointer(kv).await {
            Ok(Some(current)) => {
                keep.insert(current);
            }
            Ok(None) => {}
            Err(err) => {
                console_error!("snapshot sweep skipped, pointer unreadable: {err}");
                return;
            }
        }
        let keys = match snapshot_keys(kv).await {
            Ok(keys) => keys,
            Err(err) => {
                console_error!("snapshot sweep skipped, listing failed: {err}");
                return;
            }
        };
        let deletes = keys
            .iter()
            .filter(|key| snapshot_key_sha(key).is_some_and(|sha| !keep.contains(sha)))
            .map(|key| async move {
                if let Err(err) = kv.delete(key).await {
                    console_error!("deleting {key} failed: {err}");
                }
            });
        join_all(deletes).await;
    }
}

async fn kv_put(kv: &KvStore, write: &publish::KvWrite) -> std::result::Result<(), String> {
    kv.put(&write.key, write.value.as_str())
        .map_err(|err| err.to_string())?
        .execute()
        .await
        .map_err(|err| err.to_string())
}

/// The branch HEAD — resolved once inside the serialized run, so triggers
/// cannot race the observation.
async fn head_sha(env: &Env, repo: &str, branch: &str) -> std::result::Result<String, String> {
    let json = net::github_json(env, &head_ref_url(repo, branch)).await?;
    parse_head_ref(&json).map_err(|err| format!("head of {branch}: {err}"))
}

async fn tree_post_slugs_at(
    env: &Env,
    repo: &str,
    sha: &str,
) -> std::result::Result<Vec<String>, String> {
    let json = net::github_json(env, &tree_url(repo, sha)).await?;
    parse_tree_listing(&json).map_err(|err| format!("tree at {sha}: {err}"))
}

/// The published snapshot's sha; `None` until the first flip. A corrupt
/// pointer fails the run rather than rebuilding against the wrong index.
async fn read_pointer(kv: &KvStore) -> std::result::Result<Option<String>, String> {
    let json = kv
        .get(CURRENT_KEY)
        .text()
        .await
        .map_err(|err| err.to_string())?;
    json.map(|json| {
        CurrentPointer::from_json(&json)
            .map(|pointer| pointer.sha)
            .map_err(|err| err.to_string())
    })
    .transpose()
}

/// The previous index; missing means nothing published yet.
async fn read_index(
    kv: &KvStore,
    prev_sha: Option<&str>,
) -> std::result::Result<Vec<IndexEntry>, String> {
    let key = index_key_at(prev_sha);
    let json = kv.get(&key).text().await.map_err(|err| err.to_string())?;
    json.map(|json| serde_json::from_str(&json).map_err(|err| err.to_string()))
        .transpose()
        .map(Option::unwrap_or_default)
}

/// Every `snapshot:` key, fully paginated before any deletion.
async fn snapshot_keys(kv: &KvStore) -> std::result::Result<Vec<String>, String> {
    let mut names = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let mut list = kv.list().prefix(SNAPSHOT_KEY_SPACE.to_string());
        if let Some(cursor) = cursor.take() {
            list = list.cursor(cursor);
        }
        let page = list.execute().await.map_err(|err| err.to_string())?;
        names.extend(page.keys.into_iter().map(|key| key.name));
        match (page.list_complete, page.cursor) {
            (false, Some(next)) => cursor = Some(next),
            _ => break,
        }
    }
    Ok(names)
}
