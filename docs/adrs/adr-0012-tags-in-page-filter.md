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
