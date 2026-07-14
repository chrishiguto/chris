# PRD: Design-System Migration (v2 presentation layer)

> Companion documents: `docs/adrs/adr-0011-client-side-theming.md`,
> `docs/adrs/adr-0012-tags-in-page-filter.md` — summarized inline below. The design source
> is the claude.ai/design project `a9bc2927-53d3-4e51-9268-1606e6c253b1` (tokens under
> `tokens/*.css`, reference mock at `ui_kits/website/index.html`); every decision here was
> resolved in a full design interview on 2026-07-09.

## Problem Statement

The blog's engineering pipeline is real, but its presentation layer is a placeholder: a
generic serif theme (Lora/Libre Baskerville, blue accent) that reflects no identity, has no
user-facing theme control, no about page, and a tag-browsing IA (`/tags`, `/tags/{tag}`)
that treats post metadata as standalone pages nobody visits.

A complete design system already exists — purpose-built for this site in claude.ai/design:
a warm-tonal near-monochrome palette with one dusty-rose accent, Geist/Geist Mono
typography, a lowercase terminal-flavored voice (`~/chris` wordmark, `// note` labels,
`$ ls` empty states), restrained micro-motion, and mocks for home, writing, post, and about
screens. It is authored as a static React/CSS mock and must be translated into this
codebase's actual stack — Leptos 0.8 islands, Tailwind v4, SSR on Workers behind a
tag-purged edge cache — without disturbing the content-pipeline invariants (KV stores
meaning, deployed code owns presentation, caches are purged).

## Solution

Adapt, not clone: extract the design system's tokens and visual language into the existing
Tailwind v4 `@theme` layer, rebuild the site chrome and pages to the design's shape on the
existing SSR foundation, and add exactly four small islands for the interactivity the
design calls for (theme toggle, tag filter, code copy, easter egg). The React mock is
scaffolding and is discarded.

Concretely: Geist everywhere (sans prose/headings, mono chrome) served from Google Fonts;
the warm palette converted to oklch and declared once via `light-dark()`; a persistent
light/dark toggle with no flash and zero cache variance; a redesigned IA — `~/chris`
wordmark, "writing"/"about" nav, terminal breadcrumb on post pages, a footer, a new static
`/about` page, home showing the latest three posts; tags reworked from standalone SSR pages
into in-page client-side filtering with URL-hash state; computed read time and formatted
dates in listings and post meta; callouts restyled to two hue families with mono `// kind`
labels; code blocks gaining a chrome bar and copy button; and the design's motion grammar
(fade-up stagger, sliding link underlines, cursor blink) applied site-wide with
`prefers-reduced-motion` honored.

## Success Metrics

- The kitchen-sink post renders every AST node type and every callout kind correctly in
  **both** themes — the QA gate before shipping each slice.
- No flash of the wrong theme on any load; an explicit theme choice survives across visits;
  with no choice made, the site follows the system preference.
- Workers Cache behavior is unchanged: one cached response per URL, no new variance, the
  same `site`/`views`/`post:{slug}` tag scheme (minus the deleted tag pages).
- `just size` stays green with no new warnings — the four islands together add only a small
  wasm delta.
- `just check` and `just test` pass throughout; a content-only publish still works
  end-to-end untouched.
- Old `/tags*` URLs return 404 (deliberate); `/about` is routed, cached under the `site`
  tag, and sitemapped.

## User Stories

**Readers — theme**

1. As a reader, I want the site to follow my OS light/dark preference by default, so that
   it looks right without any action.
2. As a reader, I want a theme toggle in the header, so that I can override the system
   preference.
3. As a reader, I want my theme choice remembered, so that the site stays how I set it on
   every visit.
4. As a returning reader, I want no flash of the wrong theme while a page loads, so that
   navigation feels stable.

**Readers — reading experience**

5. As a reader, I want body text in a clean sans face at a comfortable measure (~65ch,
   17px), so that long posts are pleasant to read.
6. As a reader, I want structural chrome (nav, dates, tags, labels, code) in a mono face,
   so that the terminal voice of the site is consistent.
7. As a reader, I want each post to show a formatted date and an estimated read time, so
   that I can decide when to read it.
8. As a reader, I want code blocks with a header bar naming the language, so that I can
   orient before reading code.
9. As a reader, I want a copy button on code blocks with visible "copied" feedback, so that
   I can grab a snippet in one click.
10. As a reader, I want callouts whose kind is visible at a glance (`// note`, `// tip`,
    `// warning`, `// danger`) with severity readable through intensity, so that asides
    don't shout in four rainbow colors.
11. As a reader with `prefers-reduced-motion` set, I want transform animations disabled,
    so that the site respects my settings.

**Readers — navigation and IA**

12. As a reader, I want a home page that introduces the author and shows the latest three
    posts with a link to everything, so that I can start reading immediately.
13. As a reader, I want the post count in the "read all posts" link, so that I know the
    size of the archive.
14. As a reader, I want a writing page listing every post with title, description, date,
    and read time, so that I can scan the archive in one place.
15. As a reader, I want to filter the writing page by tag instantly, without a page load,
    so that browsing by topic feels immediate.
16. As a reader, I want a filtered view to have a shareable URL (`/posts#rust`), so that I
    can link someone to a topic.
17. As a reader, I want tag pills on a post to take me to the writing page pre-filtered on
    that tag, so that I can find related posts.
18. As a reader on a post page, I want a terminal breadcrumb (`~/chris/posts/{slug}`) and a
    "back to all posts" link, so that I always know where I am and how to get back.

    > **Amendment (2026-07-12)**: the breadcrumb ships in the article body (see Chrome
    > below) and replaces the "back to all posts" link — two stacked up-navigation rows
    > read as clutter, and the breadcrumb's linked `posts` segment is the way back.
19. As a reader, I want an about page with a short bio, what the author is currently into,
    and contact links, so that I can learn who writes this.
20. As a reader, I want a footer on every page, so that the site feels finished and signed.
21. As a reader who hits a dead URL, I want a 404 page in the site's own voice, so that
    even errors feel designed.
22. As a curious reader, I want the konami code hinted in the footer to actually do
    something, so that the site rewards attention.

**Readers — degraded modes**

23. As a no-JS reader, I want the full post list, all post content, and both nav variants
    to render server-side, so that nothing essential requires wasm.
24. As a no-JS reader, I want the site to follow my system theme, so that the missing
    toggle costs me nothing.
25. As a feed subscriber, I want titles and content in the feed to match the site exactly
    (no CSS-only casing tricks), so that the feed is not a second-class rendering.

**Author**

26. As the author, I want lowercase to be an authoring convention rather than a CSS
    transform, so that acronyms (CI, KV, MDX) keep their casing when I choose and the tab
    title, feed, and page never disagree.
27. As the author, I want read time computed from the AST at publish, so that I never
    author or maintain it.
28. As the author, I want dates authored as ISO and formatted for display, so that sorting
    stays lexicographic while readers see `jul 04, 2026`.
29. As the author, I want the callout contract unchanged (`kind` + optional `title`), so
    that no existing post needs edits.
30. As the author, I want frontmatter unchanged (no new required fields), so that the
    authoring contract in CONTENT.md stays stable.

**Operator**

31. As the operator, I want theming to add zero server variance, so that the edge cache
    stays one-response-per-URL.
32. As the operator, I want the tag-route deletion to shrink the routed/sitemapped/purged
    surface, so that the pipeline gets simpler, not just different.
33. As the operator, I want the wasm size gate to stay green, so that islands never
    threaten the Workers limit.
34. As the operator, I want pure logic (word count, date formatting, index building) tested
    natively, so that `just test` covers the new behavior without a browser.
35. As the operator, I want the docs (ADR-0008's `views` description, DOCS_INDEX) amended
    in the same change, so that specs stay authoritative.

## Implementation Decisions

**Token layer** (Tailwind v4 `@theme`, in the app's stylesheet):

- Colors convert from the design's hex to oklch and are declared once via `light-dark()`;
  `data-theme` on the root flips `color-scheme` (ADR-0011). Naming is role + numeric
  suffix: `surface`/`surface-2`/`surface-3`, `ink`/`ink-2`/`ink-3`, `line`/`line-2`,
  `accent`/`accent-2`/`accent-subtle`, `danger`. Selection and focus-ring stay plain
  (non-utility) custom properties. The four `--hue-*` callout tokens are deleted.
- Non-color namespaces keep Tailwind's default token names, re-valued to the design scale:
  text `xs 12 / sm 13 / base 16 / lg 18 / xl 22 / 2xl 28 / 3xl 38 / 4xl 52`; leading
  `tight 1.15 / snug 1.4 / normal 1.55 / relaxed 1.75`; tracking `tight −0.02em /
  wide 0.06em`; `--shadow-sm`/`--shadow-md` re-valued warm (colors via `light-dark()`
  inside the value); `--ease-out`/`--ease-in-out` re-valued and `--ease-out-expo` added;
  a `fade-up` keyframe under `--animate-*`. Radii already match Tailwind defaults
  (`md/lg/xl/full`); the spacing scale is untouched. The 17px/65ch reading measure lives
  with the prose rules.
- The serif/heading font stacks are deleted; `--font-sans` (Geist) and `--font-mono`
  (Geist Mono) remain.

**Document shell**: Geist + Geist Mono load from Google Fonts (`display=swap`) via a
`leptos_meta` stylesheet plus preconnect — the self-hosted faces, `@font-face` blocks, and
font preloads are removed. A blocking inline head script applies the stored theme before
the stylesheet (ADR-0011). The base document title becomes `~/chris`.

**Chrome**: the header carries the `~/chris` wordmark, lowercase mono nav
("writing" → `/posts`, "about" → `/about`), the active-link accent underline, and the
`ThemeToggle` island; on post pages it switches to the breadcrumb variant
(`~/chris/posts/{slug}`) from route awareness — real path segments, not the mock's fake
"blog". A footer renders on every page (copyright line + konami hint) and hosts the konami
island. The toast easter egg ships with the hint as a package.

> **Amendment (2026-07-10)**: "switches to the breadcrumb variant" shipped as the whole bar
> switching — the mono nav disappeared on post pages, leaving `/about` unreachable and
> nothing that reads as navigation. Only the wordmark gives way to the breadcrumb now; the
> nav links and toggle render on every page.

> **Amendment (2026-07-12)**: the breadcrumb left the bar entirely. Mixing it into the
> header coupled it to the wordmark — which stays the site's mark (eventually an image),
> not a path root. The bar now carries the wordmark on every page, and the post article
> opens with the breadcrumb (`~/chris / posts / {slug}`) in the body, replacing the
> "back to all posts" link.

**Pages**: home becomes greeting + intro (with animated-underline links) + "latest writing"
+ three most recent + "read all {n} posts →". The writing page is the pill row (tag-filter
island, ADR-0012) over the server-rendered post list, each row in the design's PostRow
shape (title with hover arrow, mono date · read time, truncated description, `data-tags`).
`/about` is a new hardcoded component — prompt motif, prose, "currently" list, contact
block (github + linkedin + email, all mocked until real URLs exist) — cached under the
`site` tag and sitemapped as a static page. The 404 page restyles into the same voice. Both
tag routes and their components are deleted.

**Post presentation**: article header shows title, then a mono meta row (formatted date ·
read time); tag pills move to the bottom of the article and link to `/posts#tag`. Code
blocks gain the chrome bar (language label or `code`) and a zero-prop `CopyButton` island
that reads the adjacent code text from the DOM — the code is never serialized twice.

> **Amendment (2026-07-10)**: the copy button is now part of a `CodeBlock` component and
> takes the source as a prop instead of reading the adjacent DOM — component cohesion wins
> over the never-serialized-twice rule, whose cost gzip absorbs (the prop sits right next
> to the rendered copy).
Callouts keep all four kinds and the optional title but collapse to two hue families
(note/tip → accent; warning/danger → danger) with a mono `// kind` label row; the
component-error style re-points at `danger`.

**Content crate**: `IndexEntry` gains optional `reading_minutes`, following the
`description` precedent exactly (additive serde, skip-when-absent, no schema bump, absent
values simply don't render). Two pure functions join the crate: an AST word-counter
(~200 wpm, code blocks excluded) and an English month-name date formatter (no chrono
dependency). Route constants lose everything tag-shaped; the listing-pages set drops
`/tags`; a static-pages notion adds `/about` for the sitemap.

**Publish crate**: populates `reading_minutes` when building index entries; the post page
computes the same number live from the AST it already holds.

**Workers**: the site worker's sitemap drops tag pages and adds `/about`; its router serves
`/about` under the `site` cache tag. The pipeline worker's purge-scope expectations follow
the routes change — the `views` tag now projects listings and feeds only.

**Islands inventory** (the complete list, all hand-rolled, no new dependencies):
`ThemeToggle`, `TagFilter`, `CopyButton`, `Konami`.

## Architectural Decisions

### Client-side theming with zero server variance

**Decision**: Theme is a pure client concern — tokens declared once via `light-dark()`,
`data-theme` flips `color-scheme`, localStorage persists, an inline pre-paint script
prevents flash; the server renders identical HTML for everyone.

**Context**: Every response is edge-cached and shared (ADR-0008); a theme toggle must not
fragment the cache.

**Key Drivers**:

- One cached response per URL is load-bearing for the whole read path.
- No flash of the wrong theme on load.
- System preference must win until the user explicitly chooses.

**Considered Options**:

1. Client-side `data-theme` + `light-dark()`: chosen.
2. Cookie + SSR theming: fragments the cache or kills it with `Vary`.
3. `prefers-color-scheme` only: no user control.
4. Duplicated dark token blocks (design-literal): triple declarations for the same result.

**Chosen Option**: Option 1, because it is the only model where theming costs the caching
architecture nothing.

**Trade-offs**:

- Good: cache untouched; each color declared once; choice persists.
- Good: no-flash by construction (pre-paint script).
- Bad: old browsers without `light-dark()` get light only; no-JS gets system theme only.

**See also**: `docs/adrs/adr-0011-client-side-theming.md`.

### Tags move into the writing page; SSR tag routes deleted

**Decision**: Delete `/tags` and `/tags/{tag}` end-to-end; tag browsing becomes a filter
island over the server-rendered post list with URL-hash state (`/posts#rust`).

**Context**: Tags are post metadata with no standalone-page value, and the writing page
already server-renders every post — the filter's data set is the DOM itself.

**Key Drivers**:

- Instant filtering is the design's interaction model.
- No index data serialized to the client; no rendering logic duplicated in wasm.
- Cache-key space must not grow (hash never reaches the server).

**Considered Options**:

1. SSR tag pages with pill links: filter costs a navigation; keeps useless pages.
2. Filter island + delete routes: chosen.
3. Island and routes both: two sources of truth.
4. Island owns list rendering via serialized props: data shipped twice, markup in wasm.

**Chosen Option**: Option 2, because the SSR HTML already contains everything the filter
needs, and deletion shrinks the routed/purged surface instead of growing it.

> **Amendment (2026-07-10)**: re-chosen as option 4 — the island owns the whole filter
> region (pills, rows, empty state) with the listed posts as serialized props, so
> filtering is signal state instead of DOM-attribute manipulation, and `data-tags` is
> gone from the row markup. The hash contract and the no-JS SSR baseline stand. See
> ADR-0012's amendment for the reasoning and measured costs.

**Trade-offs**:

- Good: instant, shareable, cache-invisible filtering; smaller pipeline surface.
- Bad: old tag URLs 404 (accepted — staging content, no redirects owed).
- Bad: revisit when pagination arrives; a one-page client filter stops being complete.

**See also**: `docs/adrs/adr-0012-tags-in-page-filter.md`.

## Testing Decisions

- **What makes a good test**: external behavior only — the rendered contract and the pure
  functions, never island DOM mechanics or CSS.

  > **Amendment (2026-07-14)**: "never CSS" is refined to never CSS *values*. The suite
  > keeps structural stylesheet contracts a compiler can't check — the `@import` bundle
  > holds together, every class the components emit keeps a selector (and the converse
  > via the kitchen-sink fixture), color tokens stay declared-once via `light-dark()`,
  > and the shell orders the theme script ahead of stylesheets. Visual treatments,
  > scales, and rule contents stay out of tests entirely: pinning them made the suite a
  > second copy of the stylesheet, maintained in lockstep. How things look is the
  > kitchen-sink QA read in both themes.
- **Which modules will be tested**: the content crate's word counter, date formatter, and
  routes changes (native unit tests beside the existing routes tests); the publish crate's
  index building with `reading_minutes` (extending its existing pure tests); listing
  components after the tag-route deletion (the existing router-free component-prop pattern);
  serde round-trips proving absent `reading_minutes` behaves like the `description`
  precedent.
- **Prior art**: the routes-module unit tests and the router-free `TagListing` prop pattern
  are the templates; islands follow the Counter precedent — behavior verified manually
  through the kitchen-sink QA read in both themes, not through a browser harness.

## Out of Scope

- **Syntax highlighting** — both the design and the current CSS are deliberately plain
  mono; a future highlighting pass is a `CodeBlock` presentation concern (ADR-0002 keeps
  it swappable).
- **Pagination** — the writing page renders all posts until that stops being reasonable;
  the tag-filter model is revisited then (ADR-0012).
- **A `lang` frontmatter field** — considered for the design's en/pt-br badges and
  rejected; no per-post language metadata, English-only date formatting.
- **Self-hosted fonts** — the CDN + `display=swap` choice deliberately reverses the v1
  self-hosted/`optional` strategy; revisit only if the swap flash ever bothers.
- **Redirects for deleted tag URLs**, real contact URLs and email (mocked until domains
  and handles exist), fence-metadata filename labels (no parser extensions), search,
  comments, analytics, and ADR-0010's worker deletion.

## Further Notes

- Implementation lands as three PRs, each gated on `just check` + `just test` +
  `just size` and a kitchen-sink read in both themes: (1) design-system core — tokens,
  fonts, prose/callout/code restyle, no pipeline touch; (2) IA + islands — header,
  breadcrumb, footer, about, home, theme toggle, copy button, konami, motion, date
  formatting; (3) tags rework + read time — the only content/pipeline PR.
- ADR-0008's `views`-tag description and the AGENTS.md architecture summary are amended
  inline in PR 3, per the specs-are-authoritative rule.
- The current two posts are disposable staging content; nothing in this migration edits
  them, and the lowercase voice applies to future writing as an authoring convention.
