# Deploying the pipeline worker

How to deploy `workers/pipeline` (the write-path worker, ADR-0006), wire it to
CI, and verify publishing end-to-end. One-time setup; afterwards the worker
just runs.

## What it does

Every content mutation runs through one **publish coordinator** — a single
Durable Object instance (ADR-0009) that serializes reconciles. The worker
exposes exactly one route.

`POST /publish` is the only entry point. CI calls it after any deploy (or
straight away for a content-only push). It authenticates the `Authorization:
Bearer` token against `PUBLISH_SHARED_SECRET` (401 otherwise), reads
`{"repository", "branch"}` (a reconcile always converges to HEAD), runs **one
reconcile-to-HEAD synchronously** inside the coordinator, and returns the
outcome as JSON so the calling Actions run reflects what actually happened —
no fire-and-forget. Infra errors return 500; the run fails and re-running
retries. The single-instance DO serializes overlapping calls; an Actions step
has no delivery window to protect, so the reconcile runs inline rather than
behind an alarm.

A **reconcile** (always converging to HEAD as observed at its start):

1. Resolves the branch HEAD sha, lists the post tree at that commit, and
   fetches + validates every source against the compiled component manifest.
   Reconciles are full rebuilds — there is no delta to mis-apply, so a
   duplicate or racing call is harmless by construction.
2. Writes an immutable snapshot (`snapshot:{sha}:post:*`, then
   `snapshot:{sha}:index`) and flips the single `current` pointer — the
   publish is atomic from the reader's side. A post that fails validation
   rides in as its previously published payload (or stays out if it never
   published).
3. Retains the last 10 snapshots (rollback depth) and sweeps older ones. The
   coordinator does **not** purge the cache: `cache.purge` only evicts the
   cache of the entrypoint that runs it, so a purge over a service binding
   no-ops against the site's cache (see Cache and purge below).
4. Returns a `PublishOutcome`: `{published, failed, carried, tags, ok,
   summary}`. `tags` is the stale cache-tag scope — the added/removed/changed
   posts' `post:{slug}` plus the shared `views` tag, diffed from the index
   `content_hash`es, empty when nothing changed — which CI purges from the
   site over HTTP. `ok` is false when a post failed validation: pages stale or
   missing behind a green check is the incident class this guards against.

The CI half is one workflow, `.github/workflows/publish.yml`:

- **`pull_request`** → a `check` job runs `just check` + `just test`, so a
  content or code PR is validated against its **own** branch's manifest and
  shown in the PR's checks *before* merge.
- **`push` to main** → a `publish` job declaring `environment: content`. A
  paths filter decides code-vs-content. If the push touched code it runs
  check/test, builds, gates the size budget (fail > 10 MB gzipped, warn
  > 5 MB), deploys the site, purges the `site` cache tag, and deploys the
  pipeline. It then **always** calls `/publish`, purges the stale-tag scope the
  outcome reports from the site's public `/__purge` (retried; a hard failure
  fails the job → break-glass `just purge`), and fails the job when the
  outcome's `ok` is false (or the call errored). Because the job declares
  `environment: content`, GitHub records a deployment on the merged PR that
  links straight to this run — the run, with its steps and the `/publish`
  summary, is the whole post-merge observability surface. There is no preview
  URL and no custom deployment API call.

Ordering: deploy precedes `/publish`, so content referencing new code
validates against the freshly deployed manifest. The `site`-tag deploy purge
is defensive — Workers Cache is documented as keying on the deployed version
(deploys start cold by construction), but until that is verified in
production the explicit purge guarantees no stale HTML hydrates against a new
wasm build.

## Cache and purge (ADR-0008 as amended)

The site is fronted by Workers Cache — worker-scoped, zone-free, checked
before the worker runs. Consequences for this worker:

- **Every purge runs *inside* the site worker** — Workers Cache is private
  to its owner; no REST API, zone token, or wrangler command can reach it.
  The one door is the site's `POST /__purge` (gated by the
  `PURGE_SHARED_SECRET` both workers and CI hold, constant-time check),
  taking `{"tags":[…]}`; a bodyless request means `["site"]`, the
  break-glass full purge (`just purge` wraps it).
- **Responses are tagged** (`Cache-Tag`): every cacheable page carries
  `site`, post pages add `post:{slug}`, and the index-backed views
  (listings, tag pages, feeds) add `views` — names defined once in
  `content/src/routes.rs` beside the paths they tag. Tagging is fail-closed:
  tags are the only handle a purge gets on a cached entry, so a tag set that
  can't form a valid header leaves the response uncached (loudly) rather
  than cached unpurgeable.
- **Publish purges are scoped, and run from CI.** Index entries carry a
  `content_hash` of the serialized post payload; the coordinator diffs the
  previous index against the new one and returns exactly the
  changed/added/removed posts plus `views` as the outcome's `tags`. Post N
  never evicts post M. CI purges that scope by POSTing the site's public
  `/__purge` (`just purge "<tags>"`) — the coordinator can't, because
  `cache.purge` only evicts the entrypoint that runs it and over the service
  binding that is the pipeline's, not the site's. CI retries for propagation
  and fails the run on a hard failure (break-glass: `just purge`); the 7-day
  `s-maxage` TTL stays the last-resort backstop.
- **Deploys purge `site`** from CI right after the site deploy (see above —
  defensive until version-keyed cold starts are verified in production).

Verification: watch the `Cf-Cache-Status` response header (`HIT` / `MISS` /
`BYPASS`) with `curl -sI`. Storage is per-colo (a page is only cached in
colos that served it); purge is global via Instant Purge. `wrangler dev`
resolves the `cloudflare:workers` `cache` import but (as of wrangler 4.108)
stubs it without `purge`, so `/__purge` answers 502 locally — auth and
routing are still testable; the purge itself verifies in production.

## Repo Actions configuration

- Secrets: `CLOUDFLARE_API_TOKEN` (Workers Scripts: Edit + Workers KV
  Storage: Edit), `PUBLISH_SHARED_SECRET` (same value as the worker secret
  below), and `PURGE_SHARED_SECRET` (the value the site worker holds — CI uses
  it for both the deploy `site`-tag purge and the post-publish content purge).
- Variables: `CLOUDFLARE_ACCOUNT_ID` (required); `SITE_URL` and
  `PIPELINE_URL` (optional — without them the workflow derives the
  `chris-site.<subdomain>.workers.dev` / `chris-pipeline.<subdomain>
  .workers.dev` URLs from the account's workers.dev subdomain).
- Environment: a GitHub environment named `content`. It is created
  automatically the first time the `publish` job runs; leave it with no URL
  and no protection rules (a required reviewer would gate every publish).

## Prerequisites

- The KV namespace id is pasted into **both** `wrangler.toml` (site) and
  `workers/pipeline/wrangler.toml` — the pipeline writes what the site reads.
- The coordinator Durable Object needs no manual provisioning: its binding
  and `new_sqlite_classes` migration live in `workers/pipeline/wrangler.toml`
  and ship with the deploy (SQLite-backed classes are the recommended kind
  and the only kind on the Workers Free plan).
- Local secret files (gitignored, see `.secrets/`):
  - `.secrets/github_pipeline_token` — fine-grained PAT, **Contents RO** only
    (the reconcile reads post sources at HEAD; nothing else).
  - `.secrets/publish_shared_secret` — shared with the `PUBLISH_SHARED_SECRET`
    Actions secret; authenticates CI's `/publish` call.
  - `.secrets/purge_shared_secret` — held by the **site** worker as
    `PURGE_SHARED_SECRET` (and by CI); authenticates the CI and break-glass
    `/__purge` calls. The pipeline no longer holds it.

## Deploy

```sh
just deploy-pipeline
wrangler secret put GITHUB_TOKEN --config workers/pipeline/wrangler.toml < .secrets/github_pipeline_token
wrangler secret put PUBLISH_SHARED_SECRET --config workers/pipeline/wrangler.toml < .secrets/publish_shared_secret
# the site worker holds the purge secret; CI and break-glass use it to call /__purge
wrangler secret put PURGE_SHARED_SECRET < .secrets/purge_shared_secret
```

GitHub credentials live **only** in the pipeline worker; the site worker holds
nothing but the purge secret, which grants nothing beyond a cache flush. The
pipeline no longer holds the purge secret at all — the purge is a CI concern.

## Verify content publish

1. Edit `content/blog/{slug}/index.mdx` on a branch and open a PR: the
   `check` run validates it and reports a green check on the PR (no publish —
   nothing happens to the live site from a branch).
2. Merge the PR: the `publish` run reconciles within its runtime; a
   "deployed to content" entry appears on the merged PR linking to the run,
   and the post renders at `/posts/{slug}` with the change.
3. Delete a post directory and merge: the post 404s and leaves `/posts` (a
   reconcile is a full rebuild — a post absent from HEAD is retired, no
   removal bookkeeping involved).
4. Merge a post with a broken component (`<OrbitSimulatr>`): the pre-merge
   `check` run is red, so it never reaches main under normal flow; if forced
   through, the `publish` run's reconcile returns `ok: false` (the run and
   deployment go red) and the post keeps serving its previously published
   version (carry-forward), or stays unpublished if it never published — the
   rest of the tree publishes normally.

## Verify the code path

1. Open a PR touching a co-located island
   (`content/blog/{slug}/components.rs`, see `content/blog/ci-code-path/` and
   CONTENT.md) plus its post: the `check` run builds and validates it.
2. Merge: the `publish` run builds, gates the size, deploys both workers, and
   calls `/publish`; the reconcile publishes the post with its island
   hydrating. The deployment on the merged PR links to the run, whose steps
   show the deploy and the `/publish` summary.
3. A run can be re-fired by hand from the Actions tab (re-run jobs); the
   reconcile converges to HEAD regardless of how many times it runs.

## Operations

- **Rollback**: `current` is the only mutable key; the coordinator retains
  the last 10 snapshots. To roll back, point it at a retained snapshot and
  purge (or just push a revert — a reconcile converges to HEAD either way):

  ```sh
  npx wrangler kv key put --binding BLOG --remote current '{"sha":"<retained sha>"}'
  ```

  Note the next reconcile (any merge to main) flips back to HEAD — a rollback
  that should stick must land in git.
- **Legacy key cleanup** (one-time, after the first reconcile flips
  `current`): the pre-snapshot flat keys are dead — delete `index`,
  `pending`, and every `post:{slug}`; the site reader's flat-key fallback
  only exists for the window before the first flip.
- **Migrating off the webhook era** (one-time): delete the repo's `push`
  webhook (it now 404s against the removed `/webhook` route) and its
  `GITHUB_WEBHOOK_SECRET` worker secret, and reduce the fine-grained PAT to
  **Contents RO** — the `workflow_dispatch` (Actions RW) and deployment-record
  (Deployments RW) scopes are no longer used.
- **Coordinator state**: the DO stores only its snapshot history. Deleting the
  object's storage is safe — the next publish re-seeds nothing it needs and the
  reconcile is idempotent.
