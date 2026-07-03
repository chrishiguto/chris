# ADR-0005: Proc-macro component registry with emitted manifest

**Status**: Accepted (2026-07-03)
**Related**: PRD `docs/prds/prd-leptos-workers-blog-v1.md`; consumed by ADR-0002 (render dispatch) and ADR-0004

## Context

The renderer must turn `Component{name: "OrbitSimulator", props: {"gravity": "3.71"}}` into a
typed call of a compiled Leptos component. That dispatch table — and the knowledge of which
components exist with which props — has to come from somewhere, and the same knowledge is
needed in three places: render dispatch (site worker), publish validation (pipeline worker),
and local/editor checking (`blog check`, future LSP).

## Decision

An **`#[post_component]` attribute macro**: reads the component signature, generates
string-attribute → typed-prop conversion, registers the component via the `inventory` crate
(proven on wasm32 — Leptos server functions use it under wasm-bindgen), and emits a
**machine-readable manifest** (component names, props, types, required/optional). One source of
truth, three consumers. The registry aggregates components from **both sources** — shared
components in the app crate (available to every post) and per-post co-located files
(ADR-0004) — through the same macro, manifest, and dispatch path. v1 macro scope is deliberately bounded: scalar props + children +
manifest emission; richer prop types come later.

The DX ladder this enables: `.mdx` editor highlighting (free) → `blog check` pre-commit (v1) →
diagnostics/autocomplete LSP fed by the manifest (v2; rust-analyzer can never see into
markdown, so the manifest is the only road to editor intelligence).

## Options considered

1. **Hand-written registry** — a match arm + manual prop parsing per component (~5 lines).
   Simple, debuggable, zero macro engineering; was the "ship the pipeline first" candidate.
2. **Attribute macro + inventory auto-registration** — chosen, at the user's explicit call:
   it is what makes the system feel like a framework, uses Rust the way Leptos itself does,
   and eliminates manual registry maintenance.
3. **build.rs + syn codegen** — similar power to 2, clunkier to maintain, no inline ergonomics.

## Consequences

- Good: annotate a component and you're done — no registry file to touch.
- Good: the manifest makes the vocabulary introspectable, powering validation, CLI, and LSP.
- Bad: proc-macro development and debugging cost lands in v1 (accepted knowingly).
- Bad: macro scope creep is the standing risk; the v1 boundary (scalars + children +
  manifest) is the guard rail.
