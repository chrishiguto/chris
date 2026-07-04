# Demo: hand-publish a post end-to-end (tracer bullet)

Until the pipeline worker lands (Slice 6), KV is seeded by hand. This is the
Slice 3 read-path demo: fixture `.mdx` → AST JSON → KV → `/posts/{slug}`.

## 1. Parse a fixture into AST JSON

```sh
cargo run -p content-parser --example mdx2json -- \
  content/blog/components-demo/index.mdx > /tmp/post.json
```

Any `.mdx` with `title:` and `date:` frontmatter works; diagnostics go to
stderr and exit non-zero on invalid input.

## 2. Seed KV

Post keys are `post:{slug}`. For local dev (`wrangler dev` simulates KV; the
namespace id in `wrangler.toml` is not used):

```sh
npx wrangler kv key put --binding BLOG --local "post:hello" --path /tmp/post.json
```

For production, first create the namespace once and paste its id into
`wrangler.toml`, then drop `--local` and add `--remote`:

```sh
npx wrangler kv namespace create BLOG   # once; copy the id into wrangler.toml
npx wrangler kv key put --binding BLOG --remote "post:hello" --path /tmp/post.json
```

## 3. Load the page

```sh
just dev
curl http://localhost:8787/posts/hello
```

View-source shows the full article HTML (SSR — readable without JS):
frontmatter drives `<title>`, prose nodes render as HTML, and component tags
dispatch through the registry (Slice 4) — `<Callout>` renders server-side,
`<Counter>` SSRs inside a `<leptos-island>` and hydrates in the browser. A
component name that is not registered renders a visible
`class="component-error"` span (publish validation normally rejects it long
before KV). An unknown slug (`/posts/nope`) is a plain 404 — a KV miss never
triggers any rebuild (ADR-0001).

## Theme QA (Slice 11)

`content/blog/kitchen-sink/index.mdx` is the theme's QA page: every AST node
type and every `Callout` kind in one long post. Seed it as `post:kitchen-sink`
and read it top to bottom in both color schemes (dark follows
`prefers-color-scheme`) after any theme change; it is also the page to run
Lighthouse against (PRD target: performance ≥ 95).
