# PRD: Caderno presentation layer (v3)

**Status**: Shipped (2026-09-02)

**Related**: [caderno specification, issue #47](https://github.com/chrishiguto/chris/issues/47),
`docs/adrs/adr-0008-cache-and-purge.md`,
`docs/adrs/adr-0011-client-side-theming.md`, and
`docs/adrs/adr-0012-tags-in-page-filter.md`

This document is the consolidated contract for the site that shipped. The amendment
history remains at the end so the reasons for retired paths are durable, but a reader does
not need to replay that history to reconstruct the current presentation.

## Problem statement

The engineering pipeline was specific to this site, but the presentation looked like a
generic application: a header bar, explicit theme controls, card and chip surfaces, a
terminal-flavoured voice, and separate pages for information that belongs together. The
site needs to read as one quiet notebook while preserving the runtime-content boundary:
KV stores meaning, deployed code owns presentation, and the same HTML is shared from the
edge cache.

## Goals

- Make every route feel like one sheet of warm paper in either system theme.
- Keep reading primary: restrained typography, a narrow measure, minimal chrome, and no
  decorative surface that competes with prose.
- Make `/` a compact authored index and `/writing` the complete, filterable archive.
- Offer hidden-text techniques at home and in authored posts without making core content
  depend on JavaScript.
- Keep all dates reader-facing and natural, while ISO dates remain the storage contract.
- Preserve identical server HTML per URL and the existing content, cache, and publish
  architecture.

## Design contract

### Tokens and themes

Tailwind v4 remains the styling system. Inline utilities are the default; named classes
exist only for shared design vocabulary, multi-state behaviour, pseudo-elements, or
unclassed rendered prose.

Every colour is declared once through `light-dark()`, with `color-scheme: light dark` on
the root. The palette is a low-chroma warm paper family around hue 80, three ink levels,
and one wine accent: `oklch(42% 0.13 15)` in light mode and
`oklch(70% 0.11 15)` in dark mode. Danger callouts alone use the danger hue. Selection,
focus, rules, and shadows derive from the same roles rather than adding independent
colours.

Theme follows the operating system only. There is no toggle, stored preference,
`data-theme`, pre-paint script, cookie, or server variance. A fixed, pointer-inert
pseudo-element adds static alpha-only fractal grain over the page, at lower opacity in
dark mode. Reduced-motion preferences remove nonessential transitions.

### Type

Newsreader is the single reading family. The Google Fonts request includes roman and
italic optical-size axes plus weights 300 through 700. Geist Mono is reserved for code.
The working scale is:

- body: 18px at 1.6;
- secondary text: 16px;
- minimum text: 14px;
- post title: 30px;
- section heading: 22px;
- reading measure: 44rem.

Large headings tighten tracking; quiet labels use weight, italic, or spacing rather than
an all-caps terminal idiom. Numerals that align across rows use tabular figures.

### Page frame and chrome

The wide-screen frame is flexible gutter / 44rem content / flexible gutter. Gutter
elements occupy the outer columns and fold inline below their breakpoint. The shell is a
minimum-viewport-height flex column so short pages still end at the viewport footer.

There is no header or logo. A sticky, pointer-inert six-rem veil at the top blurs by 6px
and fades from paper to transparent, giving scrolling text a soft page edge without
putting controls over it. Inner pages use a semantic sticky gutter link: `← home` on the
writing archive and `← writing` on posts. On narrow screens the link moves above the
page content.

The footer is one edge-to-edge hairline with its content aligned to the reading column.
It contains exactly the author's name and city on the left, then `rss` and `source` on the
right. There is no tagline, navigation list, theme control, or easter egg. Non-home
document titles use `{page} · ~/chris`; the home title is `~/chris`.

### Home

Route `/` is deployment-owned and cached under `site`. It renders, in order:

1. a two-line name: author, then role and city;
2. two short introduction paragraphs with inline email and source links;
3. a two-column `work` and `writing` index;
4. a dated `now` paragraph;
5. a quiet colophon line crediting the hidden-text lineage;
6. the global footer.

The introduction, career, and now copy deliberately remain placeholders for the author
to replace. They live in code, so editing them is a deploy rather than a content publish.
The work column shows role and company only until a row is hovered or focused. It keeps
the date in flow and reveals it with opacity and an eight-pixel slide, so nothing else
moves. The present stint uses the accent. A one-way fold contains the three oldest
entries. The writing column lists the latest four published titles and ends with
`all writing (N)`; it never includes descriptions or cards.

Each home section has a real visually hidden heading plus a decorative ghost word. At
64rem and wider, ghost words sit vertically in the outer gutters, with the writing word
mirrored on the right. Below that width they become small italic labels above the
section.

`/about` redirects permanently to `/`. There is no separate about page.

### Writing archive

Route `/writing` is the complete archive and is cached under `views`. A sticky `← home`
link precedes a post-count and feed line, then plain tag words and title-first rows grouped
newest-first by year. Each row reserves its word-form date in flow and reveals it on hover
or focus. No description, search field, topics rail, cards, or pills remain.

The `WritingIndex` island owns the complete archive region and receives listed posts as
serialized props. Multi-tag state lives in a sorted comma-separated `q` query, for example
`/writing?q=rust,wasm`; selection uses union semantics. Tag-word clicks use `replaceState`
without navigation, unknown tags are discarded, and empty year groups disappear with
their rows. Server rendering always emits the complete unfiltered archive, so no-JS
readers retain every post. Query variants share the same body contract and `views` cache
tag.

`/posts` redirects permanently to `/writing`, preserving its query. `/tags` and
`/tags/{tag}` do not exist. Post tag links target the filtered writing route.

### Post

Post routes remain `/posts/{slug}` and use `post:{slug}` cache tags. A post begins with a
sticky `← writing` gutter link, then a 30px medium title and a 14px italic meta line. The
meta line contains a natural-language date and, when known, `· N min`.

Rendered headings receive a wine section sign. Callouts are unfilled hairline blocks with
small-caps kind labels; note, tip, and warning use the accent, while danger uses the danger
hue. Code sits on `paper-2` behind a two-pixel left rule, with its language label and the
existing copy island.

A `.footnote` keeps the marked phrase, dagger reference, and note in server HTML. At
72rem and wider CSS moves the note into the right gutter; below that it returns inline in
italic. Posts end with unboxed tag words and no next/previous or end-of-post navigation.
The AST renderer remains unaware of routes.

### Hidden text

Three treatments let prose be read at different depths:

- A fold is a real button containing an accent ellipsis. Activating it once reveals the
  already-rendered text in place with a short fade and two-pixel rise. On home it uses a
  tiny inline script; posts use the children-only registered `Hidden` component.
- Pencil text is quiet ink with a dotted underline and darkens after a short hover delay.
- An honest edit keeps the struck phrase in flow and positions the candid insertion above
  it. Hover, focus, or touch reveals the insertion without shifting the line.

Folded text remains in the document for assistive technology, buttons carry their
expanded state, and reduced motion disables reveal transitions. The implementation is
inspired by [igorbedesqui.com](https://igorbedesqui.com/), whose lineage points to
[ped.ro](https://ped.ro/) and [lfe.org](https://lfe.org/); the home colophon carries the
same credit.

### Dates and copy

Frontmatter remains ISO `YYYY-MM-DD` for validation and sorting. Displayed dates always
read as words:

- `4 july` inside a year group;
- `4 july 2026` in standalone post metadata;
- `since 2022` for an open career range;
- `2021 to 2022` for a closed career range.

Malformed stored dates pass through instead of panicking. Site copy is lowercase except
where an authored proper name or acronym requires otherwise. Chrome and home copy use no
em dash. The retired tagline stays deleted.

### Runtime boundaries and islands

This is a presentation-layer change. The parser, AST, snapshot layout, coordinator,
publish flow, cache tags, and purge mechanism do not change. `/` carries `site`;
`/writing` and feeds carry `views`; posts carry `post:{slug}`. Redirects are uncached route
responses as specified by the worker.

The caderno presentation adds no new islands. Its two interactive island types are:

- `WritingIndex`, for archive filtering and URL state;
- `CopyButton`, for code-copy feedback.

The full shipped registry has four island types: those two, the global `Counter`, and the
co-located counter used by `ci-code-path`. Home folds and honest edits are progressive
enhancement over server HTML, implemented by the small home-local script. Everything else
is server-rendered HTML and CSS.

## Success and verification

- Every route is readable without JavaScript and receives identical HTML for the same URL.
- Light and dark system themes have no wrong-theme flash and preserve readable contrast.
- Home, writing, post, 404, redirects, feed, and sitemap follow the route and copy contracts
  above.
- The kitchen-sink post exercises every AST node, every callout kind, footnotes, and the
  `Hidden` component; it is read top to bottom in both system themes after visual changes.
- Tests assert rendered structure, accessible roles, copy, routes, and pure formatters.
  They do not pin visual CSS values.
- `just check`, `just test`, `just build`, and the worker size gate remain green.

## Deletions

The following are intentionally retired, with no compatibility layer:

- the header bar, logo, nav links, theme toggle, stored theme, and pre-paint script;
- Fraunces and Figtree, display-size type tokens, and the terminal voice;
- the konami island, toast, footer hint, and footer tagline;
- the timeline treatment, separate about page, and old writing-as-home layout;
- `/tags` pages, tag cards and pills, the topics rail, reserved search field, and clamp;
- the breadcrumb and history-based back-link island;
- filled callout cards, code chrome bar, boxed post tags, and end navigation.

## Out of scope

- Replacing the placeholder introduction, career history, or now paragraph.
- Text search, pagination, comments, analytics, or a theme override.
- Self-hosting fonts.
- Authoring pencil or honest-edit marks from MDX; only `Hidden` joins the component
  vocabulary.
- Any content-pipeline, KV-schema, publish-flow, or cache-tag redesign.

## Amendment history

This v3 text consolidates the amendments below. They remain here to explain why old code
and screenshots differ from the shipped contract.

- **2026-07-10:** the tag filter moved from DOM mutation to a `WritingIndex`-owned region
  with serialized props. `CodeBlock` began passing source to its copy island instead of
  reading adjacent DOM. Header navigation remained present on post pages while the early
  bar design was still live.
- **2026-07-12:** the post breadcrumb moved out of the header, then served as the article's
  sole upward route. This also kept navigation out of the AST renderer.
- **2026-07-14:** a theme-specific image logo replaced the text wordmark; exact-route
  current-page semantics shipped. The breadcrumb was then deleted as redundant. Tag filter
  state moved from one hash tag to a multi-tag `?q=` query with union semantics, accepting
  uniform cache-key variants under `views`. Document titles and feed title converged on the
  shared `~/chris` constant.
- **2026-07-15:** a design audit replaced all-Geist with Fraunces display, Figtree body,
  and Geist Mono labels; the terminal prompt, comment-prefix labels, and konami egg were
  removed. A history-aware `← back` island replaced the breadcrumb. Callout labels became
  tracked sans text, and the about and 404 copy left the terminal motif.
- **2026-07-16:** the writing listing temporarily became `/`, with a masthead, topics rail,
  reserved search field, and all posts. `/posts` redirected home and the nav collapsed to
  about only. This was prototype variant F and is superseded.
- **2026-09-02, caderno chrome:** issue #47 replaced the v2 visual language with warm paper,
  wine, Newsreader, static grain, the three-column page grid, top veil, gutter navigation,
  system-only theming, and the four-item footer.
- **2026-09-02, home:** the settled index home replaced writing-as-home; `/writing` became
  the full archive, `/about` redirected home, and `/posts` redirected to writing. Ghost
  words, hover dates, a career fold, pencil text, honest edits, and the dated now section
  shipped.
- **2026-09-02, writing and post:** the archive dropped its search and topics structures
  for plain tag words and year groups. Posts adopted the caderno title, meta, section sign,
  hairline callout, ruled code, margin-footnote, and tag-ending treatments.
- **2026-09-02, authoring:** the registered children-only `Hidden` component brought the
  fold to post prose and the kitchen-sink fixture. System-only theming amended ADR-0011;
  the final route/filter shape amended ADR-0012; the home/archive cache split amended
  ADR-0008.
