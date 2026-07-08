# Deploying the pipeline worker

How to deploy `workers/pipeline` (the write-path worker, ADR-0006), wire it to
GitHub, and verify both publish paths end-to-end. One-time setup; afterwards
the worker just runs.

## What it does

Every content mutation runs through one **publish coordinator** — a single
Durable Object instance (ADR-0009) that serializes reconciles. The route
handlers never touch content KV themselves; they verify, classify, and poke
the coordinator.

`POST /webhook` receives GitHub push events and:

1. Verifies the `X-Hub-Signature-256` HMAC against `GITHUB_WEBHOOK_SECRET`
   (401 on mismatch); acknowledges and ignores non-default-branch pushes.
2. Classifies the push from the commits' `added/modified/removed` paths:
   - **content-only** (`content/blog/{slug}/index.mdx` changes, no code) →
     triggers the coordinator's reconcile; live within seconds. A failed
     trigger is a 500, so GitHub webhook redelivery is the retry path.
   - **code-bearing** (any `.rs`, `app/`, `crates/`, `workers/`, `Cargo.*`,
     `justfile`, `wrangler.toml`, or workflow change) → fires the
     `publish.yml` workflow via `workflow_dispatch` with the pushed SHA and
     posts a `pending` status; the deploy must land before content
     referencing new code can validate, so CI's callback does the trigger.
   - **neither** → no-op.

`POST /publish` is CI's post-deploy callback: it authenticates the
`Authorization: Bearer` token against `PUBLISH_SHARED_SECRET` (401
otherwise), reads `{"repository", "branch"}` (CI also sends `sha`; it is
ignored — a reconcile always converges to HEAD), and triggers the same
reconcile — now against the freshly deployed manifest.

A **reconcile** (the coordinator's alarm handler, one at a time, always
converging to HEAD as observed at its start):

1. Resolves the branch HEAD sha, lists the post tree at that commit, and
   fetches + validates every source against the compiled component manifest.
   Reconciles are full rebuilds — there is no delta to mis-apply, so late,
   duplicate, or out-of-order triggers are harmless by construction.
2. Writes an immutable snapshot (`snapshot:{sha}:post:*`, then
   `snapshot:{sha}:index`) and flips the single `current` pointer — the
   publish is atomic from the reader's side. A post that fails validation
   rides in as its previously published payload (or stays out if it never
   published); the next reconcile after a deploy retries it for free.
3. Purges exactly what the publish changed from the site's Workers Cache
   (one authenticated `POST /__purge` over the `SITE` service binding, body
   `{"tags":[…]}`): the added/removed/changed posts' `post:{slug}` tags plus
   the shared `views` tag — nothing when the reconcile changed nothing.
   Retains the last 10 snapshots (rollback depth), sweeps older ones from
   KV, and posts one `blog/publish` status on the reconciled HEAD: `success`
   with the post count, `failure` naming how many posts kept previous
   versions plus the first diagnostic (the Commit Status API caps
   descriptions at 140 chars — full diagnostics via `just check`). A failed
   purge also fails the status — pages stale behind a green check is the
   incident class this guards against — and leaves its tags behind as debt:
   every later reconcile merges the debt into its own purge and clears it
   only once a purge lands, so green returns only when the cache truly
   converged. Identical repeat statuses are skipped.
4. Records one GitHub **deployment** to the `content` environment per newly
   reconciled HEAD (`success`, `environment_url` = `SITE_ORIGIN` var) — this
   is what puts "deployed to content" on the merged PR's timeline and the
   Environments panel on the repo home. Deduplicated by sha so the cron
   backstop never re-posts; best-effort like the status.
5. Re-arms its own alarm as a ~6 h cron backstop, so a missed webhook or a
   failed run self-heals without anyone pushing.

The CI half lives in `.github/workflows/publish.yml`: build both workers →
enforce the size budget (fail > 10 MB gzipped, warn > 5 MB) → deploy the
site → purge the `site` cache tag (`just purge`: a failed purge fails the
run) → deploy the pipeline → call `/publish`. The deploy purge is defensive:
Workers Cache is documented as keying on the deployed version (deploys
would start cold by construction), but that has not been verified in
production — until it is, the explicit purge guarantees no stale HTML
hydrates against a new wasm build. If verification confirms version-keyed
cold starts, the step deletes as redundant.

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
- **Publish purges are scoped.** Index entries carry a `content_hash` of the
  serialized post payload; the coordinator diffs the previous index against
  the new one and purges exactly the changed/added/removed posts plus
  `views`. Post N never evicts post M. A failed purge logs loudly, fails
  the commit status, and persists as debt the next reconcile retries; the
  7-day `s-maxage` TTL stays the last-resort backstop.
- **Deploys purge `site`** from CI right after the site deploy (see above —
  defensive until version-keyed cold starts are verified in production).

Verification: watch the `Cf-Cache-Status` response header (`HIT` / `MISS` /
`BYPASS`) with `curl -sI`. Storage is per-colo (a page is only cached in
colos that served it); purge is global via Instant Purge. `wrangler dev`
resolves the `cloudflare:workers` `cache` import but (as of wrangler 4.108)
stubs it without `purge`, so `/__purge` answers 502 locally — auth and
routing are still testable; the purge itself verifies in production.

## Repo Actions configuration (CI code path)

- Secrets: `CLOUDFLARE_API_TOKEN` (Workers Scripts: Edit + Workers KV
  Storage: Edit) and `PUBLISH_SHARED_SECRET` (same value as the worker
  secret below).
- Variables: `CLOUDFLARE_ACCOUNT_ID` (required); `PIPELINE_URL` (optional —
  without it the workflow derives the `chris-pipeline.<subdomain>
  .workers.dev` URL from the account's workers.dev subdomain).

## Prerequisites

- The KV namespace id is pasted into **both** `wrangler.toml` (site) and
  `workers/pipeline/wrangler.toml` — the pipeline writes what the site reads.
- The coordinator Durable Object needs no manual provisioning: its binding
  and `new_sqlite_classes` migration live in `workers/pipeline/wrangler.toml`
  and ship with the deploy (SQLite-backed classes are the recommended kind
  and the only kind on the Workers Free plan).
- Local secret files (gitignored, see `.secrets/`):
  - `.secrets/github_webhook_secret` — shared with the GitHub webhook config.
  - `.secrets/github_pipeline_token` — fine-grained PAT: Commit statuses RW,
    Contents RO, Actions RW (the `workflow_dispatch` trigger needs it), and
    Deployments RW (the per-publish deployment record needs it).
  - `.secrets/publish_shared_secret` — shared with the `PUBLISH_SHARED_SECRET`
    Actions secret; authenticates CI's `/publish` callback.
  - `.secrets/purge_shared_secret` — held by **both** workers as
    `PURGE_SHARED_SECRET`; authenticates the pipeline's (and break-glass
    curl's) calls to the site's `/__purge`.
- The `SITE` service binding in `workers/pipeline/wrangler.toml` names the
  deployed site worker (`chris-site`), so the site deploys first on a fresh
  account.

## Deploy

```sh
just deploy-pipeline
wrangler secret put GITHUB_WEBHOOK_SECRET --config workers/pipeline/wrangler.toml < .secrets/github_webhook_secret
wrangler secret put GITHUB_TOKEN --config workers/pipeline/wrangler.toml < .secrets/github_pipeline_token
wrangler secret put PUBLISH_SHARED_SECRET --config workers/pipeline/wrangler.toml < .secrets/publish_shared_secret
wrangler secret put PURGE_SHARED_SECRET --config workers/pipeline/wrangler.toml < .secrets/purge_shared_secret
# the site worker's half of the purge handshake — its only secret
wrangler secret put PURGE_SHARED_SECRET < .secrets/purge_shared_secret
```

GitHub credentials live **only** in this worker (user story 23); the site
worker holds nothing but the purge secret, which grants nothing beyond a
cache flush.

## Create the webhook

The local `gh` token (repo scope) can create repo hooks:

```sh
gh api repos/chrishiguto/chris/hooks -f name=web \
  -f 'events[]=push' \
  -f config[url]="https://chris-pipeline.<account>.workers.dev/webhook" \
  -f config[content_type]=json \
  -f config[secret]="$(cat .secrets/github_webhook_secret)"
```

GitHub sends a `ping` event on creation; the worker answers `pong` (a green
delivery in the hook's "Recent Deliveries" confirms the signature wiring).

## Verify the fast path

1. Edit `content/blog/{slug}/index.mdx` on `main` and push.
2. The commit on GitHub gets a `blog/publish` status within seconds
   (`success — reconciled: N posts published`); the PRD's p95 budget is
   ≤ 5 s push-to-live.
3. The post renders at `/posts/{slug}` with the change.
4. Delete a post directory and push: the post 404s and leaves `/posts` (a
   reconcile is a full rebuild — a post absent from HEAD is retired, no
   removal bookkeeping involved).
5. Push a commit with a broken component (`<OrbitSimulatr>`): the commit gets
   a red `failure` status with the first diagnostic; the post keeps serving
   its previously published version (carry-forward), or stays unpublished if
   it never published — the rest of the tree publishes normally.

Deliveries (payload + response) are replayable from the webhook's "Recent
Deliveries" tab, which is the fastest debugging loop.

## Verify the draft workflow

The two draft mechanisms (CONTENT.md "Drafts") and their purge composition:

1. **Branch = unpublished.** Author a post on a branch, push, open a PR: no
   webhook publish fires (the worker acknowledges and ignores non-default-
   branch pushes — check the delivery log), the post 404s, and listings are
   unchanged. Merge the PR: the push to `main` publishes it within seconds.
2. **`draft: true` = unlisted.** Publish a post with `draft: true`: it
   renders at `/posts/{slug}` (with `Cf-Cache-Status` never `HIT` — drafts
   answer `no-store` and are never cached), and is absent from `/`,
   `/posts`, `/rss.xml`, `/sitemap.xml`, and its tag pages.
3. **The flip.** Warm the listing caches (`curl` until `Cf-Cache-Status:
   HIT`), then push `draft: true → false`: the post appears on the listings,
   feed, sitemap, and its tag pages immediately — the publish purged the
   site's whole cache.
4. **Deletion purge.** Warm `/posts/{slug}` and the listings, then delete
   the post directory and push: the post's URL 404s and the listings drop it
   without waiting out the 7-day TTL.

## Verify the CI code path

1. Push a mixed commit: a new post plus its co-located
   `content/blog/{slug}/components.rs` island (see
   `content/blog/ci-code-path/` and CONTENT.md). The commit immediately gets
   a yellow `pending` status ("publish after the CI deploy") and a `publish`
   workflow run appears on it.
2. The run builds, gates the size, deploys both workers, and calls
   `/publish`; the reconcile posts `success` on HEAD and the post is live
   with its island hydrating. The PRD's p95 budget is ≤ 10 min push-to-live;
   both the Actions check and the `blog/publish` status sit on the same
   commit.
3. Cross-commit race drill: push a post referencing a component whose deploy
   is still in flight from a previous commit — its validation fails against
   the not-yet-deployed manifest (`failure` status, previous version kept),
   and the next reconcile (that deploy's own `/publish` callback) publishes
   it without ceremony: reconciles always rebuild from HEAD.
4. A run can be re-fired by hand from the Actions tab (`workflow_dispatch`
   takes the SHA), whose callback re-triggers the reconcile — the
   break-glass for a missed callback. The coordinator's ~6 h backstop alarm
   eventually does the same on its own.

## Operations

- **Rollback**: `current` is the only mutable key; the coordinator retains
  the last 10 snapshots. To roll back, point it at a retained snapshot and
  purge (or just push a revert — a reconcile converges to HEAD either way):

  ```sh
  npx wrangler kv key put --binding BLOG --remote current '{"sha":"<retained sha>"}'
  ```

  Note the next reconcile (any push, or the backstop alarm) flips back to
  HEAD — a rollback that should stick must land in git.
- **Legacy key cleanup** (one-time, after the first reconcile flips
  `current`): the pre-snapshot flat keys are dead — delete `index`,
  `pending`, and every `post:{slug}`; the site reader's flat-key fallback
  only exists for the window before the first flip.
- **Coordinator state**: the DO stores its reconcile target, snapshot
  history, and last posted status. Deleting the object's storage is safe —
  the next webhook re-seeds the target and the reconcile is idempotent.
