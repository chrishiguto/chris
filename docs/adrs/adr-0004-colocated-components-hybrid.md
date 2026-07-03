# ADR-0004: Co-located per-post components; content/code hybrid pipeline (Tier 2)

**Status**: Accepted (2026-07-03)
**Related**: PRD `docs/prds/prd-leptos-workers-blog-v1.md`; depends on ADR-0001, ADR-0005, ADR-0007

## Context

Posts need one-off custom components that live next to the post that owns them, without
polluting the shared design system. The design must reconcile two facts: per-post custom code
is desirable, and code cannot flow at runtime (ADR-0001). Additionally, the dream of "Rust inline in markdown with full IDE support"
founders on tooling reality: rust-analyzer only understands real `.rs` files in a cargo
workspace — inline Rust-in-markdown would have the *worst* DX, not the best.

## Decision

A post is `content/blog/{slug}/index.mdx` plus optional **`components.rs`** — a real workspace
file (full rust-analyzer) discovered by a `build.rs` scan of `content/` that emits `#[path]`
module declarations, feeding the component registry (ADR-0005). The pipeline routes per push
(ADR-0007): touches only `.mdx` → fast path, live in seconds; touches any `.rs` → CI builds,
deploys, then publishes. Principle applied consistently: **prose is content and flows
instantly; a co-located component is code and rides a deploy** — once, at authoring time,
never at read time.

## Options considered

1. **Registry-only** — all components live in shared crates; one-off experiments require
   editing the main app. Bureaucratic for the "cool widget in one post" case.
2. **Co-located `.rs` + hybrid pipeline** — chosen.
3. **Tier 3: publish-time rustc → per-post client WASM** — rejected for v1 (see ADR-0001);
   parked, and nothing in this design precludes it later.

## Consequences

- Good: one-off components stay next to their posts, with a first-class IDE story (full type
  checking across posts and the design system).
- Good: post-local code stays post-local in the repo layout.
- Bad: content pushes can trigger deploys; CI reliability joins the publish path for
  code-bearing posts.
- Bad: build-time discovery (`build.rs` + `#[path]` modules) is unavoidable machinery; the
  content tree becomes part of the build graph.
