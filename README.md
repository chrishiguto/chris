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

when i merge a pr, a github action calls the pipeline worker, which parses the post into a typed
ast, writes it to kv, and purges the urls that changed. github records it as a deployment on the
merged pr, linking straight to the run. the post is live moments later, and since publishing only
ever touches what changed, there's no site rebuild involved — publishing post number 200 costs the
same as publishing post number 2.

a post can also bring its own one-off interactive component: a `components.rs` sitting next to
the `index.mdx`, real rust with full rust-analyzer support. merges that include code do more work
first — ci builds and deploys the workers, then publishes the post. code deploys, content flows.

## the rule that holds it together

> kv stores *meaning*. deployed code owns *presentation*. caches are purged, never rebuilt.

posts are stored as a versioned, structured ast — never as html. so when i change how a code
block renders, or improve a component, every post picks it up on the next request. the stored
content doesn't even know something changed.

## how a post goes live

```
open a pr → ci validates content + code → a check shows on the pr
merge to main → one github action:
│
├─ content only  → call the pipeline: fetch → parse → validate → kv → purge (reconcile to head)
│
└─ code changed  → build → deploy workers → purge site → then call the pipeline
│
either way → a "deployed to content" deployment lands on the merged pr, linking to the run
```

## layout

```
crates/
  content/           versioned serde ast + manifest types — the contract everything
                     shares; mdx parsing + validation behind its `parse` feature
  registry/          #[post_component] macro, inventory registration, manifest
  publish/           pure publish planning: check + kv write plan (wasm-clean)
  xtask/             workspace scripts: check / plan (for just publish) / ast
app/                 leptos ui: routes, layout, ast renderer, shared components
workers/
  site/              ssr worker — leptos_axum (wasm) + axum + cache api + kv reads
  pipeline/          the /publish reconcile op (coordinator durable object) + purge
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
