# Deploying the pipeline worker

How to deploy `workers/pipeline` (the write-path worker, ADR-0006), wire it to
GitHub, and verify the webhook fast path end-to-end. One-time setup; afterwards
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
     under the KV `pending` key and stops; the CI callback drains it after
     deploy (Slice 7).
   - **neither** → no-op.
3. Posts a `blog/publish` commit status on the pushed SHA: `success` with the
   published slugs, `failure` with a concise error (the Commit Status API caps
   descriptions at 140 chars — full diagnostics via `blog check`), `pending`
   for parked code pushes.

Cache purge is a stub until Slice 8; the site's TTL bounds staleness.

## Prerequisites

- The KV namespace id is pasted into **both** `wrangler.toml` (site) and
  `workers/pipeline/wrangler.toml` — the pipeline writes what the site reads.
- Local secret files (gitignored, see `.secrets/`):
  - `.secrets/github_webhook_secret` — shared with the GitHub webhook config.
  - `.secrets/github_pipeline_token` — fine-grained PAT: Commit statuses RW,
    Contents RO (Slice 7 additionally needs Actions RW on the same PAT).

## Deploy

```sh
just deploy-pipeline
wrangler secret put GITHUB_WEBHOOK_SECRET --config workers/pipeline/wrangler.toml < .secrets/github_webhook_secret
wrangler secret put GITHUB_TOKEN --config workers/pipeline/wrangler.toml < .secrets/github_pipeline_token
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
