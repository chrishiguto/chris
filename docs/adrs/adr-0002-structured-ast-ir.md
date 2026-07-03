# ADR-0002: Versioned structured AST in KV as the content IR

**Status**: Accepted (2026-07-03)
**Related**: PRD `docs/prds/prd-leptos-workers-blog-v1.md`

## Context

JS/MDX stacks store compiled, executable artifacts (e.g. mdx-bundler emits a JS bundle
evaluated in the visitor's browser). With Leptos, the stored artifact must be pure data, and
its shape determines:
what changes force re-processing, where errors surface, what the SSR binary must contain, and
whether islands hydrate correctly.

## Decision

KV stores a **serde-typed, versioned semantic AST**: prose as structured nodes (`Heading`,
`Paragraph`, `CodeBlock{lang, text}`, `List`, `Link`, `Image`, `Component{name, props,
children}`, …), with **no pre-rendered HTML** and component references resolved **by name at
render time** through the registry. The schema lives in a shared crate (`content-ast`) with a
`schema_version` field — the single contract shared by parser, renderer, and CLI.

Corollary decisions: code blocks are stored as raw text + lang — presentation (including
syntax highlighting, server-side or client island) is a `CodeBlock` component concern,
swappable without touching stored content; component children are themselves AST (markdown
parsed recursively).

## Options considered

1. **Pre-rendered HTML** — fastest serving, but freezes both component output *and* prose
   presentation into every post (component update or heading-anchor change → re-render all
   posts: the rebuild cascade returns), and cached foreign HTML likely breaks island hydration.
2. **Structured AST** — chosen.
3. **Raw markdown, parse per request** — thinnest pipeline, but errors move to request time,
   the parser lives in the hot SSR binary, and per-request cost is paid forever.

## Consequences

- Good: the core invariant holds — *KV stores meaning, deployed code owns presentation*;
  renderer/component upgrades apply to every post instantly with zero rebuilds.
- Good: publish-time validation (unknown component = failed publish, not broken page).
- Good: SSR binary excludes the parser; the future `.rsx` static backend reuses the same IR.
- Bad: we own a schema, its versioning, and eventually a migration story.
- Bad: request-time AST walk (measured in microseconds; dominated by the KV read; further
  hidden by ADR-0008 caching).
