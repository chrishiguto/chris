# ADR-0001: Runtime content pipeline (no redeploy to publish)

**Status**: Accepted (2026-07-03)
**Related**: PRD `docs/prds/prd-leptos-workers-blog-v1.md`

## Context

The blog's defining property is: write markdown, `git push`, live in seconds — without
rebuilding the site or other posts. In the Rust/Leptos world there is a hard platform split:
Leptos components are AOT-compiled Rust, and Cloudflare Workers **forbids instantiating WASM
from runtime bytes** — no dynamic server-side code, ever. So content and code cannot share a
lifecycle the way MDX-compiled-to-JS allows in JavaScript stacks.

## Decision

Content publishes through a live pipeline (webhook → parse → KV) at runtime. Code (components,
renderer, app) rides deploys. The component *vocabulary* is fixed at deploy time; *content*
referencing that vocabulary flows instantly. Only the changed post is ever processed.

## Options considered

1. **Live pipeline** — content flows at runtime; chosen.
2. **Build-time baking** — content in the repo, rendered during `cargo leptos build`, every
   post change is a CI deploy (~minutes). Simpler and "correct" for a static blog, but kills
   the product's reason to exist and makes every typo a deploy.
3. **Publish-time compilation (Tier 3)** — a rustc container compiles per-post Rust to client
   WASM at publish. Rejected for v1: deploy-grade latency anyway (rustc is not esbuild), the
   modules can never be SSR'd (platform block above), and each post duplicates runtime WASM.

## Consequences

- Good: seconds-to-live publishes; O(changed-post) work always; no rebuild cascades.
- Good: the pipeline worker is real engineering, per the project's goals.
- Bad: adding a component requires a deploy — accepted explicitly ("it's code; code deploys").
- Bad: two lifecycles must be orchestrated when one push contains both (see ADR-0007).
