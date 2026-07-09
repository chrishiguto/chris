# ADR-0011: Client-side theming — `data-theme` + `light-dark()` with zero server variance

**Status**: Accepted (2026-07-09)
**Related**: PRD `docs/prds/prd-design-system-migration.md`, ADR-0008 (cache and purge)

## Context

The design-system migration adds a user-facing light/dark toggle. Every HTML response is
cached in Workers Cache and shared by all visitors (ADR-0008), so theming must not introduce
server-side variance — a per-theme cache key space or `Vary` header would fragment or kill
the cache. Today color flips only on `prefers-color-scheme`; there is no user override and
no persistence.

## Decision

Theme is a **pure client concern**; the server renders identical HTML for everyone.

- Every color token is declared **once** via CSS `light-dark()`; the existing
  `color-scheme: light dark` declaration makes the unset state follow the system preference.
- An explicit choice sets `data-theme` on `<html>`, which merely flips `color-scheme` —
  no duplicated dark token blocks.
- The choice persists in `localStorage`; a blocking inline `<head>` script (before the
  stylesheet) re-applies it pre-paint, so there is no flash of the wrong theme.
- A hand-rolled `ThemeToggle` island flips and persists the attribute. Two-state: unset
  follows the system until the first explicit toggle. Both glyphs are server-rendered and
  CSS picks the visible one, so the button cannot flash a stale icon before hydration.
- Shadow tokens embed their colors through `light-dark()` inside the value.

## Options considered

1. **Client-side `data-theme` + `light-dark()`** — chosen.
2. **Cookie + server-rendered theme** — no flash by construction, but fragments Workers
   Cache per theme (or `Vary: Cookie` disables it) and violates ADR-0008's
   all-dynamism-in-islands rule.
3. **`prefers-color-scheme` only** (status quo) — zero JS, but no user override; the toggle
   is a product requirement.
4. **Design-literal duplicated blocks** (`[data-theme="dark"]` + a `prefers-color-scheme`
   media query fallback) — same behavior as chosen, but every dark token is declared twice
   more; `light-dark()` collapses the duplication.

## Consequences

- Good: the caching model is untouched — one cached response per URL, no new variance.
- Good: each color is declared exactly once; the token file stays half the size of the
  design source's.
- Good: system preference is respected until the user opts out, and the choice survives
  visits.
- Bad: `light-dark()` needs a modern browser (baseline 2024); older browsers get the light
  palette. Accepted for a personal blog.
- Bad: the toggle renders both glyphs into the HTML (CSS picks one) — mild markup
  duplication bought for zero hydration flicker.
- Bad: no-JS visitors get the system theme only; the toggle SSRs but is inert. Accepted.
