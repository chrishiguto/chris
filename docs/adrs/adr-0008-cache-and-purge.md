# ADR-0008: Cache API with targeted purge; deploys purge everything

**Status**: Accepted (2026-07-03)
**Related**: PRD `docs/prds/prd-leptos-workers-blog-v1.md`; depends on ADR-0002 (AST is source of truth)

## Context

Platform facts that bound the design: Cloudflare has **no cache in front of Workers** — the
worker always executes; caching means the Cache API inside the handler (hit = sub-ms early
return, no KV read, no render). The Cache API is **per-colo**; global invalidation requires the
REST purge API. **Purge-by-tag is Enterprise-only**; purge-by-URL works on all plans. And
cached SSR HTML is **coupled to the deployed binary**: it embeds hashed `/pkg/*` asset URLs and
hydration markup — served after a deploy, it means 404'd wasm and dead islands.

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
