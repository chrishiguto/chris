# ADR-0006: Two-worker topology, split on read path vs write path

**Status**: Accepted (2026-07-03)
**Related**: PRD `docs/prds/prd-leptos-workers-blog-v1.md`

## Context

JS/MDX content pipelines typically need an external runtime (a container or build service) for
bundling, plus orchestration around it. Here the parser is a Rust crate that runs *inside* a
worker — no external runtime is needed, so the only question is how to split responsibilities
across workers.

## Decision

Two workers, split along the real fault line — read vs write:

- **`site`**: Leptos SSR (workers-rs `http`/`axum` + `leptos_axum` wasm feature), reads KV
  directly, Cache API in front. Carries the AST renderer, registry, and components — nothing
  else. Holds **no secrets**.
- **`pipeline`**: webhook validation, diff-based routing, GitHub content fetch, parse,
  validate, KV writes, cache purge, Check Run reporting. Holds `GITHUB_WEBHOOK_SECRET`,
  `GITHUB_TOKEN`, and the publish shared secret.

There is deliberately no separate read-API worker ("read KV, validate, fallback" is a
function, not a service — a separate worker would be an RPC hop for no isolation gain) and no
separate webhook-router vs parser worker (with the parser in-process, there is nothing between
them to justify a service boundary).

## Options considered

1. **One worker for everything** — parser + GitHub client + all secrets in the public serving
   binary; every pipeline change redeploys serving (and purges the whole cache, per ADR-0008).
2. **Two workers** — chosen.
3. **Three+ workers (separate webhook router / parser / read API)** — workers-rs service bindings are fetch-based (no JS-class
   RPC), so each extra hop is a serialization boundary buying isolation nobody needs.

## Consequences

- Good: minimal operational surface — no containers, no Durable Objects, no extra workers;
  two scripts and a KV namespace run the whole system.
- Good: SSR binary stays lean (size limit pressure, ADR context: 10 MB gzipped budget);
  secrets never touch the public-facing worker; independent deploy cadences.
- Bad: two wrangler configs and two deploys to keep coherent (shared KV namespace id, etc.).
