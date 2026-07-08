# ADR-0008: Cache API with targeted purge; deploys purge everything

**Status**: Accepted (2026-07-03)
**Related**: PRD `docs/prds/prd-leptos-workers-blog-v1.md`; depends on ADR-0002 (AST is source of truth)

## Context

Platform facts that bound the design: Cloudflare has **no cache in front of Workers** — the
worker always executes; caching means the Cache API inside the handler (hit = sub-ms early
return, no KV read, no render). The Cache API is **per-colo**; global invalidation requires the
REST purge API. **Purge-by-tag is Enterprise-only**; purge-by-URL works on all plans. And
cached SSR HTML is **coupled to the deployed binary**: it embeds hydration markup and `/pkg/*`
asset URLs — served after a deploy, stale markup hydrates against the new wasm build and
islands break. (The asset URLs are unhashed: leptos's `hash-files` resolves the hash manifest
at runtime via `current_exe` + `std::fs`, neither of which exists in the Workers runtime.)

## Decision

- `site` fronts every page render with `cache.match` → miss: render → `cache.put`, with
  `Cache-Control: max-age=604800` (7-day TTL as a staleness backstop for missed purges).
- **Publish** purges an explicitly enumerated URL set via REST purge-by-URL (all colos):
  changed post URLs, `/`, `/posts`, `/rss.xml`, `/sitemap.xml`, the changed posts' `/tags/{t}`
  pages, `/tags`. Adding/editing a post **never** touches other posts' cache entries.
- **Deploy** runs `purge_everything` as the CI step after `wrangler deploy` — a correctness
  requirement (binary coupling above), not a freshness preference. Rare event, one API call,
  cost = one cold render per page per colo.
- Post pages are static-per-content-per-deploy; **all per-visitor dynamism lives in islands**
  that fetch client-side. KV remains the source of truth; the cache is never trusted, only
  invalidated.

*Amendment (post-v1, crate consolidation):* the purge set is no longer a hand-maintained
mirror of the site's routes. The KV keys, post/tag path builders, and the listing/feed path
sets are defined once in `content` (`content/src/routes.rs`) and consumed by the site's
router/sitemap, the app's hrefs, and the publish plan's purge list — adding an index-backed
page in one place makes it routed, listed, and purged by construction. The draft-visibility
filter is likewise one helper (`IndexEntry::is_listed`) instead of per-consumer `!draft`.

*Amendment (2026-07-07, ADR-0009):* publishes are now full snapshot rebuilds, which cannot
know which post *bodies* changed — so the publish purge set widened from "touched posts and
their tags" to the whole enumerated URL surface of the previous and new indexes (listings,
feeds, every post URL, every tag page either index knows about). Still purge-by-URL, still
chunked to the 30-file cap, still "never trust the cache, only invalidate it"; the
"post N never evicts post M" efficiency invariant is relaxed at blog scale in exchange for
convergent publishes. Revisit with per-post source hashes if the purge volume ever matters.

*Amendment (2026-07-07, browser caches):* the original `max-age=604800` reached browsers too,
and no purge can touch a client cache — returning visitors kept stale pages for up to 7 days,
breaking the visible-instantly contract. The header is now `max-age=0, s-maxage=604800`: the
edge keeps its 7-day backstop TTL (the Cache API honors `s-maxage`), browsers must revalidate
every view. To make that revalidation cheap, cacheable pages carry `ETag: "{snapshot sha}"`
(the sha the loaders already read to key KV — no extra read) and the shim answers a matching
`If-None-Match` with a bodyless 304 on both the hit and miss paths, the stored copy always
keeping its full body. A site-wide validator means any publish invalidates every page's ETag —
over-fetching, never staleness, the same trade the ADR-0009 amendment made for the purge set.
Static assets need none of this: `[assets]` serves them without invoking the worker, and
Workers Assets defaults to `public, max-age=0, must-revalidate` with a strong ETag — the same
revalidation semantics. Content-hashed asset filenames with immutable TTLs remain the
unclaimed perf upgrade (blocked on leptos hash-file resolution in wasm; see Context).

*Amendment (2026-07-07, Workers Cache):* two of the Context's founding facts fell. First,
"the Cache API is inert on workers.dev" proved false in production: `cache.put` stored fine
there while purge-by-URL had no zone to purge, so a content publish left every page stale for
the full backstop TTL — the worst combination, caching active and purge impossible. Second,
"Cloudflare has no cache in front of Workers" stopped being true on 2026-07-06, when Workers
Cache launched: a worker-scoped, zone-free cache the runtime checks *before* invoking the
worker, on any domain including workers.dev. The hand-rolled `cache.match`/`cache.put` front
is deleted; `[cache] enabled = true` fronts the worker instead, storing responses per the same
`max-age=0, s-maxage=604800` header the handlers already set (the explicit `no-store` default
on drafts, 404s, and errors is now load-bearing — Workers Cache gives unmarked responses
heuristic freshness). Cache keys include the deployed version, so every deploy starts from an
empty cache by construction — the binary-coupling hazard dissolves and the CI
`purge_everything` step, its zone gating, and the zone purge credentials are deleted (option 3
below, "build-ID cache-key generations", arrived as a platform primitive). Purging now happens
only from inside the owning worker (`cache.purge` via `cloudflare:workers` — purge-by-tag and
`purgeEverything`, no Enterprise gate, no zone); the publish-time purge becomes a
pipeline-called endpoint on the site worker, specified in ADR-0009's amendment. Until that
endpoint lands, content publishes converge via the TTL backstop or the next deploy.

## Options considered

1. **No caching** — fine (single-digit ms renders) but leaves free performance unclaimed.
2. **Cache API + URL purge + deploy nuke** — chosen.
3. **Build-ID cache-key generations** — purge-free deploys (each build reads/writes a fresh
   cache namespace, old entries age out). Elegant, but the pipeline worker must discover the
   site worker's current build ID to purge versioned keys on publish — cross-worker machinery
   unjustified at this scale. Documented as the upgrade path if `purge_everything` ever hurts.

## Consequences

- Good: hit path approaches static-site serving; miss path is self-limiting (per colo, per
  deploy).
- Good: purge scope matches the efficiency invariant (post N never invalidates post M).
- Bad: worker still executes on every request (hits are cheap, not free).
- Bad: purge credentials (zone + API token) needed in both CI (deploy nuke) and the pipeline
  worker (publish purge).
