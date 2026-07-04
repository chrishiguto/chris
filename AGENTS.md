# AGENTS.md

Guidance for AI coding agents working in this repository.

## What this is

Personal blog, Rust end-to-end: Leptos 0.8 (islands mode) SSR compiled to wasm, running in Cloudflare Workers. Cargo workspace with `app` (Leptos UI, `hydrate`/`ssr` features), `workers/site` (SSR worker), `content-ast` (versioned AST IR), `content-parser` (MDX-subset parsing + manifest validation), and `registry`/`registry-macro` (`#[post_component]` dispatch + manifest; leptos only under its `dispatch` feature — keep it that way so parser-side consumers stay lean). Remaining crates (`blog-cli`, `workers/pipeline`) are specced in the PRD but not yet built. The authoring format contract is `CONTENT.md`.

## Specs are authoritative

`docs/prds/` and `docs/adrs/` are the source of truth; `docs/DOCS_INDEX.md` is the index. When implementation diverges from or extends an ADR/PRD, amend the doc inline as part of the same change (see ADR-0007's amendment for the pattern) and keep `DOCS_INDEX.md` in sync.

## Commands

All builds route through `just` (wrangler.toml's `[build]` also calls `just build` — never bypass it):

- `just dev` — wrangler dev at http://localhost:8787
- `just build` — cargo-leptos build (client) then worker-build (SSR worker)
- `just size` — gzipped wasm sizes; the server artifact (`workers/site/build/index_bg.wasm`) must stay under the Workers 10 MB gzipped limit
- `just deploy` — wrangler deploy
- `just fmt` — leptosfmt (view! macros, config in `leptosfmt.toml`) + cargo fmt
- `just check` — fmt-check + `cargo clippy --workspace -- -D warnings` (runs on the native target — it only compiles because the ssr deps in `workers/site` are optional and feature-gated; keep them that way)

## Load-bearing pins and gotchas (do not "fix" these)

- `worker = "=0.8.3"` hard pin: 0.8.4+ pulls wasm-streams 0.6 alongside server_fn's 0.5 and fat LTO fails on duplicate wasm-bindgen shims. Unpin only once server_fn catches up.
- Two getrandom majors (0.3 and 0.4) are in the graph; each needs the `wasm_js` feature AND `--cfg getrandom_backend="wasm_js"` in wasm RUSTFLAGS (the justfile sets this — reuse its recipes rather than raw cargo for wasm builds).
- No `strip = true` in the release profile — it breaks wasm-bindgen (cloudflare/workers-rs#1014); wasm-opt strips after bindgen instead.
- `LEPTOS_OUTPUT_NAME` must stay `chris` everywhere (workspace metadata `output-name`, justfile).
- `workers/site/build/` (including its `package.json`) is generated output, not source.

## Git

Conventional commits (`feat:`/`fix:`/`docs:`). Work on feature branches with PRs, not directly on main.
