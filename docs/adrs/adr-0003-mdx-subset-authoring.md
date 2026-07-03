# ADR-0003: MDX-syntax subset authoring format; props are data, not code

**Status**: Accepted (2026-07-03)
**Related**: PRD `docs/prds/prd-leptos-workers-blog-v1.md`

## Context

MDX's authoring feel (prose + JSX-style component tags) is the benchmark for rich markdown
authoring. But real MDX is markdown **+ JavaScript** — imports, expressions, code in props. We
can never evaluate JS; whatever the format is, component props must be pure data.

## Decision

Posts are `.mdx` files in an **MDX-syntax subset**, parsed by the `markdown` crate
(markdown-rs) in MDX mode — no custom parser:

- Markdown prose as usual; frontmatter (YAML) for metadata.
- PascalCase tags are component invocations; lowercase tags pass through as HTML.
- Component children are markdown, parsed recursively.
- **Props are scalar literals only** (strings, numbers, booleans) in v1; structured data
  arrives as children or is deferred.
- `import` / `export` / `{js expressions}` are **rejected at publish time** with a clear
  diagnostic explaining the subset (component names resolve via the registry instead).
- The subset is documented in `CONTENT.md`.

## Options considered

1. **MDX subset** — chosen: parser exists, editors highlight `.mdx` today, authoring parity
   with MDX itself.
2. **Markdown directives** (`:::name{attr=…}`) — honest about not being MDX, but requires
   parser extension work and has no editor support.
3. **Hugo-style shortcodes** (`{{< name >}}`) — same drawbacks, worse ergonomics.

## Consequences

- Good: zero parser engineering; the familiar MDX writing experience carries over.
- Good: publish-time validation makes unsupported constructs loud, not surprising.
- Bad: it looks like MDX but isn't — without `CONTENT.md` discipline this will confuse
  future authors (including future-us).
- Bad: no expression props; components needing rich config must design around literals and
  children until a structured-prop story is added.
