# AGENTS.md

Guidance for AI coding agents working in this repository.

## What this is

Personal blog, Rust end-to-end: Leptos 0.8 (islands mode) SSR compiled to wasm, running in Cloudflare Workers. Cargo workspace with `app` (Leptos UI, `hydrate`/`ssr` features; its `build.rs` discovers co-located `content/blog/*/components.rs` as modules per ADR-0004), `workers/site` (SSR worker fronted by Workers Cache — responses carry `Cache-Tag`s (`site`; `views` for the listings and feeds — tag browsing is an in-page filter island riding the URL hash per ADR-0012, not routed pages; `post:{slug}`; all defined in `content/src/routes.rs`), purged by tag only from inside the worker via its secret-gated `POST /__purge`; ADR-0008 as amended), `content` (the shared vocabulary crate: versioned AST IR + component-manifest types, wasm-lean by default; MDX-subset parsing + manifest validation behind its `parse` feature — keep read-path consumers on the default feature set so they never link markdown-rs), `registry`/`registry-macro` (`#[post_component]` dispatch + inventory registration producing a `content::Manifest`; leptos only under its `dispatch` feature — keep it that way, and it re-exports the manifest types so macro-generated `::registry::…` paths stay stable), `publish` (pure publish planning: check + immutable snapshot plan under `snapshot:{sha}:*` keys; wasm-clean — no fs/HTTP/clock — so the pipeline worker can reuse it), `authn` (the shared constant-time bearer-token check both workers gate their internal endpoints on — the site's `/__purge` and the pipeline's `/publish`; pure, natively tested), `xtask` (the workspace scripts bin, cargo-xtask pattern: `check` validates the content tree, `plan` lays the whole tree out as one wrangler-ready snapshot for `just publish`, `ast` prints one post's AST JSON; transport is wrangler's, never its own — see `docs/guides/publishing.md`), and `workers/pipeline` (the write-path worker: one authenticated `POST /publish` that runs a reconcile-to-HEAD synchronously in a single publish-coordinator Durable Object and returns the outcome — snapshot write, `current` pointer flip, retention, and the stale cache-tag scope (changed posts + views, diffed via index `content_hash`es) as `tags` for CI to purge; a validation failure makes the outcome `ok: false` (failing the calling run); the coordinator itself no longer purges — `cache.purge` is scoped to the entrypoint that runs it, so a purge over a service binding no-ops against the site's cache; ADR-0009 as amended. Pure decision logic is native-testable, the wasm shim sits behind its `worker` feature — see `docs/guides/pipeline-deploy.md`). CI is one workflow, `.github/workflows/publish.yml`: `pull_request` runs `just check` + `just test` (pre-merge validation, shown in the PR's checks); `push` to main runs one job (`environment: content`) that deploys the workers when a paths filter sees code (build → size gate → deploy site → purge the `site` cache tag → deploy pipeline; the deploy purge is defensive until Workers Cache's version-keyed cold starts are verified in production) and then always calls `/publish` and purges the stale-tag scope it returns from the site's public `/__purge` over HTTP (only the site worker as top-level entrypoint can evict its own Workers Cache), failing the job when the outcome is not `ok` or the purge ultimately fails — GitHub records that job's deployment on the merged PR, linking to the run, which is the observability. The authoring format contract is `CONTENT.md`.

## Specs are authoritative

`docs/prds/` and `docs/adrs/` are the source of truth; `docs/DOCS_INDEX.md` is the index. When implementation diverges from or extends an ADR/PRD, amend the doc inline as part of the same change (see ADR-0007's amendment for the pattern) and keep `DOCS_INDEX.md` in sync.

## Commands

All builds route through `just` (wrangler.toml's `[build]` also calls `just build` — never bypass it):

- `just dev` — wrangler dev at http://localhost:8787
- `just build` — cargo-leptos build (client) then worker-build (SSR worker)
- `just size` — gzipped wasm sizes; fails when a worker script (`workers/site/build/index_bg.wasm` or the pipeline's) exceeds the Workers 10 MB gzipped limit, warns past 5 MB (CI runs the same recipe)
- `just deploy` — wrangler deploy
- `just publish` — manual/break-glass content publish: the whole tree as one snapshot + `current` pointer flip (`xtask plan` → `wrangler kv bulk put` → `kv key put current`; `remote='--local'` targets the dev simulator)
- `just fmt` — leptosfmt (view! macros, config in `leptosfmt.toml`) + cargo fmt
- `just check` — fmt-check + `cargo clippy --workspace -- -D warnings` + `xtask check` content validation (runs on the native target — it only compiles because the ssr deps in `workers/site` are optional and feature-gated; keep them that way)

## Load-bearing pins and gotchas (do not "fix" these)

- `worker = "=0.8.3"` hard pin: 0.8.4+ pulls wasm-streams 0.6 alongside server_fn's 0.5 and fat LTO fails on duplicate wasm-bindgen shims. Unpin only once server_fn catches up.
- The `#[durable_object]` macro emits bindgen glue with bare `wasm_bindgen::` paths, so any crate hosting a Durable Object (today: `workers/pipeline`) must keep a direct `wasm-bindgen` dependency — removing it breaks only the wasm build, which `just check` doesn't compile.
- Two getrandom majors (0.3 and 0.4) are in the graph; each needs the `wasm_js` feature AND `--cfg getrandom_backend="wasm_js"` in wasm RUSTFLAGS (the justfile sets this — reuse its recipes rather than raw cargo for wasm builds).
- No `strip = true` in the release profile — it breaks wasm-bindgen (cloudflare/workers-rs#1014); wasm-opt strips after bindgen instead.
- `LEPTOS_OUTPUT_NAME` must stay `chris` everywhere (workspace metadata `output-name`, justfile).
- `workers/site/build/` (including its `package.json`) is generated output, not source.

## Git

Conventional commits (`feat:`/`fix:`/`docs:`). Work on feature branches with PRs, not directly on main.
