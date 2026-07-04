# Deploying the pipeline worker

How to deploy `workers/pipeline` (the write-path worker, ADR-0006), wire it to
GitHub, and verify both publish paths end-to-end. One-time setup; afterwards
the worker just runs.

## What it does

`POST /webhook` receives GitHub push events and:

1. Verifies the `X-Hub-Signature-256` HMAC against `GITHUB_WEBHOOK_SECRET`
   (401 on mismatch); acknowledges and ignores non-default-branch pushes.
2. Classifies the push from the commits' `added/modified/removed` paths:
   - **content-only** (`content/blog/{slug}/index.mdx` changes, no code) →
     publishes immediately: fetches the changed sources from the GitHub
     contents API at the pushed SHA, validates against the compiled component
     manifest, writes `post:{slug}` + rewrites `index` in KV, deletes removed
     posts.
   - **code-bearing** (any `.rs`, `app/`, `crates/`, `workers/`, `Cargo.*`,
     `justfile`, `wrangler.toml`, or workflow change) → parks the publish set
     under the KV `pending` key and fires the `publish.yml` workflow via
     `workflow_dispatch` with the pushed SHA; a failed dispatch is a 500, so
     GitHub webhook redelivery is the retry path.
   - **neither** → no-op.
3. Posts a `blog/publish` commit status on the pushed SHA: `success` with the
   published slugs, `failure` with a concise error (the Commit Status API caps
   descriptions at 140 chars — full diagnostics via `blog check`), `pending`
   for parked code pushes.

`POST /publish` is CI's post-deploy callback (ADR-0007's ordering guarantee):

1. Authenticates the `Authorization: Bearer` token against
   `PUBLISH_SHARED_SECRET` (401 otherwise; user story 35). The body carries
   `{"sha", "repository"}` from the workflow run.
2. Drains the KV `pending` list: each changed entry's source is fetched at
   its *own* pushed SHA and validated individually; removals become KV
   deletes. One merged plan rewrites `post:*` keys and `index`.
3. Entries that fail validation stay parked and retry on the next callback —
   this is the cross-commit race resolution (user story 32). A source that
   404s at its SHA (force-push) parks the same way instead of wedging the
   rest; any later push touching that slug supersedes it.
4. Posts a `blog/publish` status per pushed SHA in the drained set, so every
   parked commit's page reflects its own content's fate.

The CI half lives in `.github/workflows/publish.yml`: build both workers →
enforce the size budget (fail > 10 MB gzipped, warn > 5 MB) → deploy site +
pipeline → cache purge → call `/publish`. The purge step only runs when the
`CLOUDFLARE_ZONE_ID` repo variable is set: `purge_everything` needs a zone,
and on `*.workers.dev` there is none (the Cache API is inert there too), so
until a custom domain lands the step is skipped by design.

## Cache purge (ADR-0008)

Two purge layers, different scopes, deliberately ordered:

- **Deploy** (`publish.yml`, above): `purge_everything` right after
  `wrangler deploy` — cached HTML embeds hashed `/pkg/*` URLs and hydration
  markup coupled to the deployed binary, so a deploy invalidates all of it.
  On the CI code path this runs *before* the `/publish` callback, so the
  callback's targeted purge is never undone by the nuke.
- **Publish** (this worker): after every applied KV plan, REST purge-by-URL
  of exactly the plan's enumerated set (`publish_core::PublishPlan::purge`):
  `/`, `/posts`, `/rss.xml`, `/sitemap.xml`, `/tags`, the touched posts'
  URLs, and the tag pages they appear on (old and new tags). Publishing
  post N never evicts post M. Requests are chunked to the API's 30-files
  cap; failures log loudly but never fail the already-applied publish — the
  site's 7-day TTL backstops a missed purge.

Publish purge configuration (all in `workers/pipeline/wrangler.toml` /
worker secrets; empty = purge skipped with a log line, correct for
workers.dev where the Cache API is inert):

- `CLOUDFLARE_ZONE_ID` var — the custom domain's zone.
- `SITE_ORIGIN` var — the site's absolute origin (e.g.
  `https://blog.example.com`); purge-by-URL matches full URLs exactly, and
  the site keys its cache entries on the same bare `origin + path` shape
  (query strings are stripped at `cache.put` time).
- `CLOUDFLARE_PURGE_TOKEN` secret — API token scoped to Zone → Cache
  Purge → Purge for that zone.

Per-colo caveat for verification: `cache.put` is per-colo, purge is global.
A page is only *cached* in colos that have served it, so "post-purge serves
fresh content" should be spot-checked from ≥ 2 regions (or accept the
single-colo check: a purged URL re-renders on its next request anywhere).
The `x-blog-cache: hit|miss` response header the site sets makes this
observable with `curl -sI`.

## Repo Actions configuration (CI code path)

- Secrets: `CLOUDFLARE_API_TOKEN` (Workers Scripts: Edit + Workers KV
  Storage: Edit) and `PUBLISH_SHARED_SECRET` (same value as the worker
  secret below).
- Variables: `CLOUDFLARE_ACCOUNT_ID` (required); `CLOUDFLARE_ZONE_ID` and
  `PIPELINE_URL` (both optional, for the custom-domain future — without
  `PIPELINE_URL` the workflow derives the `chris-pipeline.<subdomain>
  .workers.dev` URL from the account's workers.dev subdomain).

## Prerequisites

- The KV namespace id is pasted into **both** `wrangler.toml` (site) and
  `workers/pipeline/wrangler.toml` — the pipeline writes what the site reads.
- Local secret files (gitignored, see `.secrets/`):
  - `.secrets/github_webhook_secret` — shared with the GitHub webhook config.
  - `.secrets/github_pipeline_token` — fine-grained PAT: Commit statuses RW,
    Contents RO, and Actions RW (the `workflow_dispatch` trigger needs it).
  - `.secrets/publish_shared_secret` — shared with the `PUBLISH_SHARED_SECRET`
    Actions secret; authenticates CI's `/publish` callback.

## Deploy

```sh
just deploy-pipeline
wrangler secret put GITHUB_WEBHOOK_SECRET --config workers/pipeline/wrangler.toml < .secrets/github_webhook_secret
wrangler secret put GITHUB_TOKEN --config workers/pipeline/wrangler.toml < .secrets/github_pipeline_token
wrangler secret put PUBLISH_SHARED_SECRET --config workers/pipeline/wrangler.toml < .secrets/publish_shared_secret
```

Secrets live **only** in this worker (user story 23); the site worker stays
credential-free.

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
   (`success — published {slug}`); the PRD's p95 budget is ≤ 5 s push-to-live.
3. The post renders at `/posts/{slug}` with the change.
4. Delete a post directory and push: the post 404s and leaves `/posts`.
5. Push a commit with a broken component (`<OrbitSimulatr>`): the commit gets
   a red `failure` status with the first diagnostic; KV is untouched.

Deliveries (payload + response) are replayable from the webhook's "Recent
Deliveries" tab, which is the fastest debugging loop.

## Verify the draft workflow

The two draft mechanisms (CONTENT.md "Drafts") and their purge composition:

1. **Branch = unpublished.** Author a post on a branch, push, open a PR: no
   webhook publish fires (the worker acknowledges and ignores non-default-
   branch pushes — check the delivery log), the post 404s, and listings are
   unchanged. Merge the PR: the push to `main` publishes it within seconds.
2. **`draft: true` = unlisted.** Publish a post with `draft: true`: it
   renders at `/posts/{slug}` (with `x-blog-cache: miss` on every request —
   drafts are never cached), and is absent from `/`, `/posts`, `/rss.xml`,
   `/sitemap.xml`, and its tag pages.
3. **The flip.** Warm the listing caches (`curl` until `x-blog-cache: hit`),
   then push `draft: true → false`: the post appears on the listings, feed,
   sitemap, and its tag pages immediately — the publish purged exactly those
   URLs plus `/posts/{slug}`.
4. **Deletion purge (Slice 8 composition).** Warm `/posts/{slug}` and the
   listings, then delete the post directory and push: the post's URL 404s
   and the listings drop it without waiting out the 7-day TTL.

Steps 3–4 exercise real purging only behind a custom domain with
`CLOUDFLARE_ZONE_ID`/`SITE_ORIGIN`/`CLOUDFLARE_PURGE_TOKEN` configured (see
"Cache purge"); on workers.dev the Cache API is inert and every response is
a `miss`, so freshness holds trivially.

## Verify the CI code path

1. Push a mixed commit: a new post plus its co-located
   `content/blog/{slug}/components.rs` island (see
   `content/blog/ci-code-path/` and CONTENT.md). The commit immediately gets
   a yellow `pending` status ("parked for CI publish") and a `publish`
   workflow run appears on it.
2. The run builds, gates the size, deploys both workers, and calls
   `/publish`; the status flips to `success` and the post is live with its
   island hydrating. The PRD's p95 budget is ≤ 10 min push-to-live; both the
   Actions check and the `blog/publish` status sit on the same commit.
3. Cross-commit race drill: push a post referencing a component whose deploy
   is still in flight from a previous commit — its validation fails, the
   entry stays parked (`failure` status, "parked for retry"), and the next
   callback (that deploy's own workflow run) publishes it.
4. A run can be re-fired by hand from the Actions tab (`workflow_dispatch`
   takes the SHA), which re-drains whatever is pending — the break-glass for
   a missed callback.
