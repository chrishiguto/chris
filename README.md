# chris

my corner of the internet. a blog, but mostly an excuse to over-engineer a blog.

everything here is rust: [leptos](https://leptos.dev) ssr compiled to wasm, running inside a
cloudflare worker at the edge. there's no node, no containers, no queues — the whole system is
two worker scripts and a kv namespace.

## the idea

i write posts in markdown with real leptos components dropped into the prose. it looks like
mdx, but the components are compiled rust:

```mdx
---
title: building an orbit simulator in rust
date: 2026-07-02
tags: [rust, wasm]
---

gravity assists sound like cheating. here's the intuition, in plain markdown.

<OrbitSimulator gravity="3.71" planet="Mars" />

that widget is live — drag the slider. it's a leptos island, hydrated on the client.

<Callout kind="warning">
  children are markdown too, parsed recursively.
</Callout>
```

when i push, a worker picks up the webhook, parses the post into a typed ast, writes it to kv,
purges the urls that changed and reports back as a commit status on the commit. the post is live a
couple of seconds later, and since publishing only ever touches what changed, there's no site
rebuild involved — publishing post number 200 costs the same as publishing post number 2.

a post can also bring its own one-off interactive component: a `components.rs` sitting next to
the `index.mdx`, real rust with full rust-analyzer support. pushes that include code take a
different path — ci builds and deploys first, then publishes the post. code deploys, content
flows.

## the rule that holds it together

> kv stores *meaning*. deployed code owns *presentation*. caches are purged, never rebuilt.

posts are stored as a versioned, structured ast — never as html. so when i change how a code
block renders, or improve a component, every post picks it up on the next request. the stored
content doesn't even know something changed.

## how a post goes live

```
git push → webhook → pipeline worker
│
├─ only content/**/*.mdx changed
│    → fast path: fetch → parse → validate → kv → purge → commit status.  live in ~2s.
│
└─ any .rs / app code changed
     → ci: build → deploy workers → purge everything → publish the pending posts.
       ordering comes free from ci being sequential — no distributed state machine.
```

## layout

```
crates/
  content-ast/       versioned serde ast — the contract everything shares
  content-parser/    markdown-rs mdx mode → ast + validation diagnostics
  registry/          #[post_component] macro, inventory registration, manifest
  blog-cli/          blog check / blog publish --local
app/                 leptos ui: routes, layout, ast renderer, shared components
workers/
  site/              ssr worker — leptos_axum (wasm) + axum + cache api + kv reads
  pipeline/          webhook + publish op + github checks + purge
content/
  blog/{slug}/index.mdx [+ components.rs]
```

## running it

prereqs: [rust](https://rustup.rs), node (wrangler is a node cli — the only node here),
and [`just`](https://github.com/casey/just) (`cargo install just`). then:

```sh
just setup     # once: wasm target, cargo-leptos, worker-build
just dev       # build everything and serve at http://localhost:8787
just deploy    # ship to workers.dev
just size      # gzipped wasm sizes (the workers plan limit cares)
```

`just` plays the role package.json scripts play in js-land: a plain command runner, recipes
in the [justfile](justfile). the build pipeline is defined exactly once — `just build`
compiles the frontend islands + tailwind via cargo-leptos and the ssr worker via
worker-build — and `wrangler.toml` calls that same recipe on dev and deploy, so there's no
second copy of the build to drift.

## docs, and how this is built

this project is spec-driven and built in collaboration with ai (claude code, mostly). the flow
goes: we design together in long architecture sessions, decisions land in a
[prd](docs/prds/prd-leptos-workers-blog-v1.md) and [adrs](docs/adrs) with the reasoning
preserved, the prd gets broken down into vertical-slice [issues](../../issues), and
implementation follows that paper trail. there's a [docs index](docs/DOCS_INDEX.md) too, so the
ai can find its way around the same way you would.

## references & inspirations

- [kent c. dodds](https://github.com/kentcdodds) — his blog is a big inspiration for what this
  wants to be. his is react on a very different infrastructure, but the spirit — a personal
  site that's also a serious piece of engineering — comes from there.
- [matt pocock](https://github.com/mattpocock) — the spec-driven, ai-collaborative workflow
  here is inspired by his approach. i didn't adopt it wholesale though — i brought my own
  beliefs into the flow along the way.
