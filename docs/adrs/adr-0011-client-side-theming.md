# ADR-0011: System theming via `light-dark()` with zero server variance

**Status**: Accepted (2026-07-09; amended 2026-09-02)
**Related**: PRD `docs/prds/prd-design-system-migration.md`,
[caderno specification](https://github.com/chrishiguto/chris/issues/47), ADR-0008 (cache and purge)

## Context

Every HTML response is cached in Workers Cache and shared by all visitors (ADR-0008), so
theming must not introduce server-side variance. The original decision met that constraint
with a client-side override: `data-theme`, localStorage, a pre-paint script and a toggle
island. The caderno design removes visible application chrome and makes system preference
the complete theme contract; retaining an invisible stored override would make the page
surprising and leave dead machinery behind.

## Decision

Theme follows the system only, and the server renders identical HTML for everyone.

- Every color role is declared once via CSS `light-dark()`.
- `color-scheme: light dark` on the root lets the user agent select the system scheme.
- There is no `data-theme` override, persisted choice, pre-paint script, or toggle island.
- Palette-dependent values, including selection, focus, and shadow colors, use the same
  declared-once mechanism.

This amends the original explicit-override decision. Cache variance remains zero while the
client code and hydrated-island inventory shrink.

## Options considered

1. **System preference + `light-dark()`** — chosen; no script, persistence, flash, or cache
   variance.
2. **Client override + `light-dark()`** — previously chosen; preserves reader control but
   conflicts with the no-chrome design and requires persistence plus pre-paint machinery.
3. **Cookie + server-rendered theme** — fragments Workers Cache per theme (or disables it
   through `Vary: Cookie`).
4. **Duplicated `prefers-color-scheme` token blocks** — zero script, but declares every
   dark token a second time for behavior `light-dark()` already expresses.

## Consequences

- Good: one cached response per URL and one declaration per color role remain intact.
- Good: served markup has no visitor-specific state and cannot flash a stored stale choice.
- Good: deleting the toggle and its pre-paint path reduces client wasm and chrome.
- Trade-off: readers cannot override their operating-system preference on this site.
- Trade-off: browsers without `light-dark()` support receive the light palette; accepted
  for this personal site.
