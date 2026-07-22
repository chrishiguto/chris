# ADR-0012: Tag browsing moves into the writing page; SSR tag routes deleted

**Status**: Accepted (2026-07-09)
**Related**: PRD `docs/prds/prd-design-system-migration.md`, ADR-0008 (cache and purge)

## Context

v1 shipped `/tags` (index with counts) and `/tags/{tag}` (filtered listing) as SSR pages:
routed, sitemapped, cached under the `views` tag, purged on publish. The design system
presents tags as filter pills *inside* the writing page instead, and the standalone pages
have no reader value — tags are post metadata, useless outside the listing context. The
writing page already server-renders **every** post (there is no pagination), so the full
data set a filter needs is always present in the DOM as HTML.

## Decision

Delete both tag routes end-to-end (routes, components, sitemap entries, purge-scope
references, tests). Tag browsing becomes a small **filter island on the writing page**:

- Post rows stay server-rendered HTML, each carrying its tags as a data attribute; the
  island owns only the pill row and toggles row visibility via the DOM. No index data is
  serialized to the client and no rendering logic is duplicated in wasm.
- Filter state lives in the **URL hash** (`/posts#rust`): shareable, restored on load, and
  invisible to the server — Workers Cache still sees exactly one `/posts` page.
- Tag pills on a post page link to `/posts#that-tag`, landing on the pre-filtered listing.
- An empty filter result shows a mono `$ ls`-style empty state.

> **Amendment (2026-07-10)**: the island now owns the whole filter region — pill row, post
> list, and empty state — with the listed posts serialized as island props (option 4 below,
> originally rejected). Filtering over SSR'd rows meant the island talked to server HTML
> through DOM selectors and attribute contracts (`data-tags`, tags read back out of pill
> hrefs) instead of signals — the framework's reactivity was unused exactly where state
> changes. At this site's scale the rejected costs are noise: a listed post serializes to
> ~200 bytes of props and the row markup adds little to a worker already ~750 KB gzipped
> against the size gate's 5 MB warning. What stands unchanged: filter state lives in the
> URL hash, the server and cache still see exactly one `/posts` page, and the island's own
> server render keeps the complete list visible without JS.

> **Amendment (2026-07-14)**: filter state moves from the URL hash to the `q` query
> parameter and becomes a multi-tag selection — `/posts?q=rust,wasm`, comma-separated and
> sorted (the slug-only tag grammar keeps the comma unambiguous), a row showing when it
> carries *any* selected tag. The fragment was the wrong slot for view state: `?q=` is the
> conventional shape, and a single hash never generalized to a selection. What this trades
> away: the query string, unlike the fragment, reaches the server, so Workers Cache can now
> hold one entry per deep-linked selection instead of exactly one `/posts` page. Each such
> entry is the same unfiltered SSR body (filtering stays client-side; pill clicks
> `replaceState` and never navigate, so only shared or reloaded links mint new entries), all
> carry the `views` tag, and publishes purge them together — the fragmentation is unbounded
> in count (any query string mints an entry here, as on every cached route) but uniform in
> body and purge-safe. Old `/posts#tag` deep links land on the unfiltered listing, not a 404.

ADR-0008's `views` tag description narrows accordingly: it now covers the listings and
feeds only (there are no tag pages left to project the index).

## Options considered

1. **Keep SSR tag pages, pills as links** — zero JS and cache-friendly, but keeps pages
   judged useless, and every filter click costs a navigation.
2. **Filter island over SSR rows + delete the routes** — chosen.
3. **Island *and* SSR routes both** — two sources of truth for the same view, double the
   surface for no reader benefit.
4. **Island owns the list rendering** (index serialized as island props) — ships the data
   twice (HTML + props) and moves list markup into wasm; the size gate exists for a reason.

## Consequences

- Good: filtering is instant and matches the design; the wasm addition is DOM toggling
  only.
- Good: the routed/sitemapped/purged surface shrinks; `LISTING_PAGES` drops `/tags`.
- Good: hash state costs nothing in cache-key space and makes filtered views shareable —
  which the design mock's ephemeral client state never was.
- Bad: old tag URLs 404. Accepted: current content is staging-only, so no redirects are
  owed; the lost tag landing pages were a negligible SEO surface.
- Bad: filtering needs JS. No-JS readers still see the complete list (it is the SSR
  baseline); the pills simply do nothing.
- Bad: if the listing ever paginates, a client-side filter over one page is no longer
  complete — server-side filtering returns as part of that work, not this one.

> **Amendment (2026-07-16)**: writing became the home. `/` now carries the listing and the
> filter island directly, so the `?q=` selection roots at `/` (`/?q=rust,wasm`) instead of
> `/posts`, and `tag_filter_path*` builds those. The bare `/posts` path is no longer a
> listing route: it `301`s to `/` at the site worker (preserving any `?q=`), `LISTING_PAGES`
> shrinks to just `/`, and the sitemap lists `/` (not `/posts`). Post pages are unchanged at
> `/posts/{slug}`. The island also grew the writing page's structure — a topics rail of tag
> pills beside the list, and a reserved (inert) search field above it — but its contract is
> unchanged: server render is the full unfiltered list, pill clicks `replaceState` and never
> navigate, and every deep-linked `?q=` entry is the same SSR body under the `views` tag.
> Old `/posts?q=` links land filtered via the redirect; old `/posts` links land on `/`.
