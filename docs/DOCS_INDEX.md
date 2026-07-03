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
  runtime via webhook→parse→KV, code rides deploys; rejects build-time baking and publish-time
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
  dispatch, publish validation, blog check CLI, and a future LSP. Topics: proc macro, registry,
  manifest, inventory, dx.
- `docs/adrs/adr-0006-two-worker-topology.md` — ADR (Accepted) — two workers split read/write:
  site (SSR + KV read, no secrets) and pipeline (webhook + publish + secrets); no containers,
  Durable Objects, or separate read-API workers. Topics: topology, workers-rs, secrets,
  binary size.
- `docs/adrs/adr-0007-publish-orchestration.md` — ADR (Accepted) — one publish operation, two
  invokers (webhook fast path; CI callback after deploy for code pushes); CI sequencing replaces
  a distributed state machine; GitHub Check Runs as the publish receipt for both paths. Topics:
  orchestration, ci, github checks, ordering, pending retries.
- `docs/adrs/adr-0008-cache-and-purge.md` — ADR (Accepted) — Cache API full-response caching
  (per-colo) with REST purge-by-URL on publish and purge_everything on deploy (hydration
  correctness); all dynamism in islands. Topics: caching, purge, cache api, deploys, islands.

## Other

- `README.md` (repo root) — project introduction in the author's voice: the publish-flow
  diagram, the core principle (KV stores meaning / deployed code owns presentation / caches
  are purged), and the target workspace layout tree. Not a spec — the PRD and ADRs above are
  authoritative.
