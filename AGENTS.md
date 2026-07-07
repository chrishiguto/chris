# AGENTS.md

Guidance for AI coding agents working in this repository.

## What this is

Personal blog, Rust end-to-end: Leptos 0.8 (islands mode) SSR compiled to wasm, running in Cloudflare Workers. Cargo workspace with `app` (Leptos UI, `hydrate`/`ssr` features; its `build.rs` discovers co-located `content/blog/*/components.rs` as modules per ADR-0004), `workers/site` (SSR worker with a Cache API front, ADR-0008), `content` (the shared vocabulary crate: versioned AST IR + component-manifest types, wasm-lean by default; MDX-subset parsing + manifest validation behind its `parse` feature — keep read-path consumers on the default feature set so they never link markdown-rs), `registry`/`registry-macro` (`#[post_component]` dispatch + inventory registration producing a `content::Manifest`; leptos only under its `dispatch` feature — keep it that way, and it re-exports the manifest types so macro-generated `::registry::…` paths stay stable), `publish` (pure publish planning: check + immutable snapshot plan under `snapshot:{sha}:*` keys; wasm-clean — no fs/HTTP/clock — so the pipeline worker can reuse it), `xtask` (the workspace scripts bin, cargo-xtask pattern: `check` validates the content tree, `plan` lays the whole tree out as one wrangler-ready snapshot for `just publish`, `pointer` reads the `current` pointer, `ast` prints one post's AST JSON; transport is wrangler's, never its own — see `docs/guides/publishing.md`), and `workers/pipeline` (the write-path worker: webhook HMAC + push classification + commit statuses + `workflow_dispatch` trigger + the authenticated `/publish` CI callback, all funneling into a single publish-coordinator Durable Object that serializes reconcile-to-HEAD publishes — snapshot write, `current` pointer flip, retention, purge-by-URL, one status per reconciled HEAD; ADR-0009. Pure decision logic is native-testable, the wasm shim sits behind its `worker` feature — see `docs/guides/pipeline-deploy.md`). The CI half of the code path is `.github/workflows/publish.yml` (build → size gate → deploy → zone-gated purge → `/publish`). The authoring format contract is `CONTENT.md`.

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
