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
`purgeEverything`, no Enterprise gate, no zone); the publish-time purge is the site worker's
secret-gated `POST /__purge` route (an HTTP route, not RPC: purge is scoped to the
cache-owning entrypoint — here the *default* public one — and workers-rs cannot author the
private named `WorkerEntrypoint` that would host a secret-free RPC purge), called by the
pipeline after every pointer flip — mechanics and rationale in ADR-0009's amendment.

*Amendment (2026-07-08, scoped tag purge):* `purgeEverything` is retired. Every cacheable
response now carries `Cache-Tag` — `site` on everything, `views` on the index-backed views
(listings, tag pages, feeds — they project every post, so any content change purges them
together), `post:{slug}` on each post page — with the names defined once in
`content/src/routes.rs` beside the paths they tag. `POST /__purge` takes `{"tags":[...]}`;
a bodyless request reads as `["site"]`, the break-glass full purge (`just purge` wraps it).
The publish purge is scoped at last: index entries carry a `content_hash` of the serialized
post payload (computed in `publish::snapshot`, recomputed for carried posts so pre-hash
entries heal in place), the coordinator diffs the previous index against the new one, and
purges exactly the added/removed/changed posts' tags plus `views` — nothing when a reconcile
changes nothing. This restores the "post N never evicts post M" invariant the ADR-0009
amendment relaxed (it is the per-post-source-hash revisit that amendment promised). Two
hardening lessons from the 2026-07-08 incident (pointer flipped, purge silently skipped on a
missing pipeline secret, pages stale for hours behind a green check): a failed purge now
makes the reconcile outcome `ok: false` (failing the publish run — originally the
`blog/publish` commit status; see ADR-0009's 2026-07-08 amendment) instead of only logging,
and CI purges `site` right
after the site deploy (via `just purge`, which fails the run loudly) — defensive, because the
version-keyed-cold-start claim in the previous amendment has not been verified in production
and cached entries were observed serving after deploys. If a controlled test (warm page →
no-op deploy → still `HIT`?) confirms deploys self-invalidate, the CI purge step deletes as
redundant; if it refutes it, the previous amendment's "empty cache by construction" claim is
wrong and this purge is load-bearing.

*Amendment (2026-07-09, content purge moves to CI over HTTP):* the coordinator's post-flip
`/__purge` call went out over the `SITE` service binding — but `cache.purge()` is scoped to the
entrypoint that *runs* it, and over a binding that is the pipeline's entrypoint, not the site's.
It no-op'd against the site's cache while returning success, so content-only merges were
green-while-stale for the full `s-maxage` (KV flipped, `purged: true`, pages served the
pre-merge snapshot). The only entrypoint that can evict the site's Workers Cache is the site
worker itself as top-level — i.e. `/__purge` over public HTTP, exactly what the post-deploy
`just purge` step already does. So the content purge moves to CI: `/publish` returns the
computed scope as `PublishOutcome.tags` (empty when nothing changed), and the workflow's
Publish step POSTs `${SITE_ORIGIN}/__purge` with those tags (`just purge "<tags>"`, retried for
Instant-Purge propagation, failing the job on a hard non-200 → break-glass `just purge`). The
in-worker purge is deleted whole: `net::purge_site`, the `SITE` binding, the
`PURGE_SHARED_SECRET` the pipeline held, and the coordinator's purge-debt ledger all go — CI's
retry covers transient failures and a hard failure goes red, so there is nothing left to carry
as debt. The `purged` field leaves `PublishOutcome`; `ok` now reflects validation only. (See
ADR-0009's 2026-07-09 amendment for the coordinator side.)

*Amendment (2026-07-10, ADR-0012):* the tag pages are gone — tag browsing moved into the
writing page as a client-side filter island, so `/tags` and `/tags/{tag}` are no longer
routed, sitemapped, or cached (`LISTING_PAGES` dropped `/tags`, `tag_path` is deleted).
The `views` tag narrows accordingly: it now covers the index-backed listings and feeds only
(`/`, `/posts`, `/rss.xml`, `/sitemap.xml`). The publish purge scope is structurally
unchanged — changed posts' tags plus `views` — it simply selects fewer cached entries.
Filter state rides the URL hash, which never reaches the cache key, so Workers Cache still
sees exactly one `/posts` page.

*Amendment (2026-07-12, deploy-aware ETags):* the snapshot-sha ETag survived code deploys —
browsers revalidating at `max-age=0` got a 304 against the unchanged validator and kept HTML
rendered by the *previous* worker until the next content publish (a presentation-only deploy
was invisible to returning readers). The validator now pairs the sha with the deployed
version: `ETag: "{sha}-{version id}"`, the id from the Version Metadata binding
(`[version_metadata]`, available in the pinned worker 0.8.3). Static pages — previously
validator-less because they read nothing from KV — carry the version alone and gain cheap
304s between deploys. Same trade as ever: over-fetching (a redeploy of identical code
re-sends bodies once), never staleness. Dev builds (`BUILD_PROFILE=dev`, compiled with
`debug_assertions`) skip cache decoration entirely, falling through to the `no-store`
default — wrangler dev's version id is a static placeholder that can never bust anything,
and watch rebuilds must not be masked by the local cache simulation or a browser validator.

Two invariants keep the scoped design honest — both close holes a future implementer could
easily reopen. First, *purge debt*: a failed purge leaves its tag scope in the coordinator's
storage; every later reconcile merges that debt into its own scope and clears it only once a
purge lands. Deriving purge success from the index diff alone is not enough — the next
reconcile of an unchanged HEAD diffs to nothing and would post green over a still-stale
cache, silently re-hiding the very failure the status exists to surface. A debt ledger that
cannot be read escalates the scope to a full `site` purge — over-purge, never staleness.
*(Retired by the 2026-07-09 amendment: with the purge moved to CI there is no in-worker purge
to fail and no debt to carry — CI retries a transient failure and reddens the run on a hard
one, where a same-HEAD re-run re-derives the same scope from git and purges again.)* Second,
tagging is *fail-closed*: `Cache-Tag` is written before `Cache-Control`, so a tag
set that cannot form a valid header value leaves the response uncached (with a loud log)
rather than cached
untagged. Tags are the only handle a purge ever gets on a cached entry — an untagged entry
is invisible to every tag purge, `site` included, and serves stale until its TTL. Slugs are
validated to `[a-z0-9-]` so this cannot fire today, but any future tag built from arbitrary
input (say `tag:{name}` from frontmatter) would hit it.

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
