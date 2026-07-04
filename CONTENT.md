# CONTENT.md — the authoring format

Posts look like MDX but are **not** MDX (ADR-0003): they are markdown plus
component *tags*, with zero JavaScript. Everything a post can express is pure
data; component behavior lives in compiled Rust, resolved by name through the
registry (ADR-0005). This file is the contract — when in doubt, `blog check`
(`cargo run -p blog-cli -- check`) and publish validation enforce exactly
what is described here.

## Where posts live

```
content/blog/{slug}/index.mdx
```

The directory name is the slug: `content/blog/components-demo/index.mdx` is
served at `/posts/components-demo`.

## Frontmatter

Every post starts with a YAML frontmatter block fenced by `---`:

```yaml
---
title: The v1 component vocabulary   # required
date: 2026-07-04                     # required, YYYY-MM-DD
description: One line for the feed.  # optional; the post's feed summary
tags: [meta, rust]                   # optional; lowercase slugs (a-z, 0-9, -)
draft: true                          # optional; keeps the post out of listings
---
```

TOML frontmatter (`+++`) and posts without frontmatter are rejected.

Each tag names its `/tags/{tag}` page verbatim, so tags must be lowercase
slugs — letters, digits, and hyphens only; `blog check` enforces this. Posts
without a `description` fall back to their title in the feed.

## Markdown

Standard markdown works as usual: headings, paragraphs, emphasis/strong,
inline code, fenced code blocks (stored as raw text + language; highlighting
is a renderer concern), ordered/unordered lists, blockquotes, links, images,
thematic breaks, and hard breaks.

Two constraints:

- **Inline links only** — reference-style links (`[text][ref]` plus a
  definition) are rejected; write `[text](url)`.
- **Lowercase HTML tags pass through** with string attributes only, e.g.
  `<abbr title="HyperText">HT</abbr>`. Their children are still markdown.

## Components

PascalCase tags invoke registered Leptos components:

```mdx
<Callout kind="warning" title="Heads up">
  Children are **markdown**, parsed recursively.
</Callout>

<Counter initial={3} />
```

Rules:

- **Names** are plain PascalCase identifiers resolved through the registry.
  `<Foo.Bar>`, `<foo:bar>`, and fragments (`<>…</>`) are rejected. A typo'd
  name fails the publish with a "did you mean" suggestion — it never renders
  a broken page.
- **Props are scalar literals only** (ADR-0003):
  - strings use quotes: `kind="warning"`
  - numbers and booleans use braces: `initial={3}`, `ratio={1.6}`,
    `enabled={true}`
  - a bare prop (`fancy`) means `true`
  - anything else in braces is code and is rejected.
- **Children are markdown**, parsed recursively; components declared without
  children reject them.

## What is rejected, and why

Posts are data, not programs. The parser rejects, with file/line diagnostics:

| Construct | Why |
|---|---|
| `import` / `export` statements | No JS: component names resolve via the registry |
| `{expressions}` in prose or props | Props are literal data, never evaluated |
| `{...spread}` attributes | Same — props must be named scalar literals |
| Unknown components / props / wrong prop types | Validation against the component manifest at publish time (user story 13) |
| Reference-style links | Keep the subset small; inline links only |
| TOML or missing frontmatter | Metadata is required, YAML only |

## The v1 vocabulary

| Component | Props | Children | What it does |
|---|---|---|---|
| `<Callout>` | `kind` (string, required), `title` (string, optional) | markdown | Highlighted aside |
| `<Counter>` | `initial` (integer, required) | none | Interactive island demo |

## Adding a component

Shared components live in `app/src/components/`; annotate a Leptos component
with `#[post_component]` (above `#[component]`/`#[island]`) and it registers
itself — name, props, and children-acceptance flow into the manifest that
validation and `blog check` consume:

```rust
#[post_component]
#[component]
pub fn Callout(kind: String, title: Option<String>, children: Children) -> impl IntoView { … }
```

v1 prop types: `String`, `f64`, `i64`, `bool`, or `Option` of one of these
(`Option` props are optional in MDX); `children: Children` if the component
wraps markdown. Anything richer is rejected at compile time (ADR-0005's
bounded scope).

Co-located per-post components (`content/blog/{slug}/components.rs`,
ADR-0004) are specced but not wired up yet; discovery via `build.rs` lands
with the CI code path (Slice 7).
