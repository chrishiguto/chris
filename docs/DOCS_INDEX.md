# Documentation Index

Machine-friendly index of project documents. One entry per document: path, type, one-line
summary, key topics.

## PRDs

- `docs/prds/prd-leptos-workers-blog-v1.md` — PRD — v1 of the Leptos SSR blog on Cloudflare
  Workers: a Rust two-worker system (site SSR + content pipeline) with instant markdown
  publishing, with success metrics, user stories, module design, KV schema, publish flows, and
  embedded ADR summaries. Topics: leptos, cloudflare-workers, ssr, blog, content
  pipeline, kv, mdx, prd.

## ADRs

- `docs/adrs/adr-0001-runtime-content-pipeline.md` — ADR (Accepted) — content publishes at
  runtime via push→parse→KV, code rides deploys; rejects build-time baking and publish-time
  rustc. Topics: publish lifecycle, wasm restrictions, instant publish.
- `docs/adrs/adr-0002-structured-ast-ir.md` — ADR (Accepted) — KV stores a versioned serde AST
  (semantic nodes + component refs by name), never HTML or raw markdown; "KV stores meaning,
  deployed code owns presentation". Topics: ast, ir, kv schema, hydration, serde.
- `docs/adrs/adr-0003-mdx-subset-authoring.md` — ADR (Accepted) — .mdx files in an MDX-syntax
  subset parsed by markdown-rs MDX mode; props are literal data; import/export/expressions
  rejected at publish time. Topics: authoring format, mdx, markdown-rs, validation.
- `docs/adrs/adr-0004-colocated-components-hybrid.md` — ADR (Accepted) — per-post components.rs
  beside index.mdx, discovered by build.rs; content-only pushes publish instantly, code pushes
  ride CI (Tier 2 hybrid). Topics: co-located components, rust-analyzer, hybrid pipeline.
- `docs/adrs/adr-0005-macro-registry-manifest.md` — ADR (Accepted) — #[post_component] proc
  macro: prop conversion, inventory registration, and a component manifest consumed by render
  dispatch, publish validation, the `xtask check` gate, and a future LSP. Topics: proc macro, registry,
  manifest, inventory, dx.
- `docs/adrs/adr-0006-two-worker-topology.md` — ADR (Accepted) — two workers split read/write:
  site (SSR + KV read, no secrets) and pipeline (publish op + secrets); no separate read-API
  worker (amended 2026-07-08: the pipeline lost its webhook/routing/status role to one Actions
  workflow, and the write path grew a Durable Object per ADR-0009). Topics: topology,
  workers-rs, secrets, binary size.
- `docs/adrs/adr-0007-publish-orchestration.md` — ADR (Accepted; ordering superseded by
  ADR-0009; trigger superseded 2026-07-08) — one publish operation; the webhook fast path +
  `workflow_dispatch` code path collapse into a single GitHub Actions workflow calling a
  synchronous `/publish`, with observability moving from commit statuses (amended from Check
  Runs — Checks API write is GitHub-App-only) to a native `content` deployment on the merged
  PR plus a pre-merge check. Topics: orchestration, ci, deployments, ordering.
- `docs/adrs/adr-0008-cache-and-purge.md` — ADR (Accepted) — full-response caching in front
  of the worker; all dynamism in islands (amended twice by ADR-0009, then re-platformed
  2026-07-07: Workers Cache replaces the hand-rolled Cache API front, purge only from inside
  the worker; amended 2026-07-08: `Cache-Tag`s `site`/`views`/`post:{slug}` replace
  purgeEverything, publishes purge only changed posts via index `content_hash` diffs, failed
  purges make the publish run fail (outcome `ok: false`) and persist as debt later reconciles
  retry, tagging is fail-closed (no valid tag header → no caching), CI purges `site` after deploys pending
  version-keying verification; amended 2026-07-09: the content purge moves to CI over HTTP —
  `cache.purge` is entrypoint-scoped so the pipeline's binding purge no-op'd, `/publish` now
  returns the stale-tag scope and CI evicts it via the site's public `/__purge`, retiring the
  in-worker purge, the `SITE` binding, the pipeline's purge secret, and the purge-debt ledger).
  Topics: caching, purge, cache tags, workers cache, deploys, islands.
- `docs/adrs/adr-0009-snapshot-publish-coordinator.md` — ADR (Accepted) — publishes are
  immutable `snapshot:{sha}:*` sets behind one `current` pointer; the publish operation is a
  reconcile-to-HEAD (full rebuild, carry-forward for invalid posts) serialized by a single
  coordinator Durable Object; pending-list machinery deleted; rollback = re-point the pointer
  (amended 2026-07-07: the purge set is deleted — the coordinator calls the site's `/__purge`
  over a service binding instead; amended 2026-07-08: one Actions workflow calls a synchronous
  `/publish`, the alarm/commit-status/deployment-API machinery is removed, and observability
  rides a native `content` deployment on the merged PR; amended 2026-07-09: the coordinator
  stops purging — the service-binding purge was a no-op (`cache.purge` runs in the caller's
  entrypoint), so it returns the stale-tag scope for CI to purge over HTTP and the `purged`
  field, purge-debt ledger, `SITE` binding, and `purge_scope` are deleted). Topics: snapshots,
  atomic publish, reconcile, durable objects, convergence, rollback, retention.
- `docs/adrs/adr-0010-ci-reconcile-drop-worker.md` — ADR (**Proposed**, not implemented) — the
  deferred "Architecture 2": now that every publish rides one Actions workflow, delete the
  pipeline worker + coordinator DO and reconcile directly from CI via `xtask` + `wrangler`
  (ADR-0009's rejected Option 3, unblocked by dropping the fast path); captured for later
  evaluation with tradeoffs and open questions. Topics: simplification, ci reconcile, proposed,
  worker removal, serialization.

## Guides

- `docs/guides/pipeline-deploy.md` — Guide — deploy the pipeline worker (including the
  coordinator DO's shipped migration), provision its secrets, and verify publishing: a
  pre-merge `check` job on PRs and a single push-to-main workflow that deploys the workers when
  code changed then calls the synchronous `/publish` (reconcile-to-HEAD), surfaced as a native
  `content` deployment on the merged PR; plus operations (rollback via the `current` pointer,
  legacy key cleanup, webhook-era migration, coordinator state). Topics: pipeline worker,
  /publish, github actions, environment deployment, deploy, secrets, ci, size budget,
  durable object, reconcile, rollback.
- `docs/guides/publishing.md` — Guide — manual publishing: `just check` (validate the content
  tree against the compiled component vocabulary) and `just publish` (break-glass/bulk
  publish: `xtask plan` lays the whole tree out as one snapshot, `wrangler kv bulk put` +
  pointer flip + purge; wrangler owns auth; bypasses the coordinator by design). Topics:
  xtask, just, publish, kv, snapshots, wrangler, break-glass, rollback.
- `docs/guides/tracer-bullet-demo.md` — Guide — hand-publish a post end-to-end before the
  pipeline worker exists: `xtask ast` → `wrangler kv key put` → `/posts/{slug}`.
  Topics: demo, kv seeding, read path, tracer bullet.

## Other

- `CONTENT.md` (repo root) — Spec — the authoring format contract: MDX-syntax subset,
  frontmatter fields, prop literal rules, rejected constructs with reasons, the current
  component vocabulary, and how to add a `#[post_component]`. Topics: authoring, mdx subset,
  components, props, validation.
- `README.md` (repo root) — project introduction in the author's voice: the publish-flow
  diagram, the core principle (KV stores meaning / deployed code owns presentation / caches
  are purged), and the target workspace layout tree. Not a spec — the PRD and ADRs above are
  authoritative.
