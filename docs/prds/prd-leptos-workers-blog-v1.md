# PRD: Leptos SSR Blog on Cloudflare Workers (v1)

> Companion documents: `docs/adrs/` — one ADR per major decision, summarized inline below.

## Problem Statement

I want a personal engineering blog where the blog itself is the engineering project: Rust
end-to-end, Leptos SSR, deployed on Cloudflare Workers. The publishing experience must be:
write a post in markdown with embedded interactive components, `git push`, live in seconds —
without rebuilding the site or any other post.

Nothing off the shelf provides this:

- Static-site generators make every edit a full rebuild and deploy — publishing a typo fix
  costs minutes of CI, and adding a post rebuilds everything.
- The JS ecosystem solves rich authoring with MDX, but its central artifact — an executable
  JS bundle evaluated in every visitor's browser — has no Rust/Leptos equivalent: Leptos
  components are AOT-compiled, and Workers forbids loading code at runtime. Rich markdown
  authoring for a Rust frontend has to be designed, not adopted.
- Dynamic blog pipelines typically carry heavy operational weight (containers, queues,
  orchestration services) for what is conceptually "parse markdown, cache it, serve it."

## Solution

A Rust workspace deployed as two Cloudflare Workers:

- **`site`** — Leptos 0.8 SSR running inside a Worker (workers-rs `http`/`axum` bridge +
  `leptos_axum` wasm feature), serving all pages. Blog posts are rendered at request time from a
  **versioned, structured AST** stored in KV: prose nodes map to HTML elements, component nodes
  dispatch through a **macro-generated registry** to real compiled Leptos components (including
  interactive islands). Full responses are cached via the Cache API.
- **`pipeline`** — receives GitHub push webhooks, decides content-vs-code by inspecting changed
  paths, and owns the single **publish operation**: fetch changed `.mdx` from GitHub, parse
  (markdown-rs MDX mode) into the AST, validate against the component manifest, write KV, purge
  affected URLs, and report a **GitHub commit status** on the commit. Pushes containing Rust code
  instead trigger the CI workflow, which deploys and then calls the same publish operation —
  ordering by CI sequentiality, no distributed state machine.

Authoring: posts are `content/blog/{slug}/index.mdx` (MDX-syntax subset — component tags with
literal props; no JS) plus optional co-located `components.rs` (real Rust, full rust-analyzer,
baked into the registry at build time). Branches are drafts; merge to `main` publishes. A
`blog check` CLI (sharing the parser/validator crates) catches errors pre-push.

The entire system is two Worker scripts and a KV namespace — no containers, no Durable
Objects, no queues.

## Success Metrics

**User-facing (the author is the primary user):**
- Fast-path publish: push of a content-only commit → post live, **≤ 5 s p95** (webhook to
  purged-and-servable).
- Code-path publish: push with new component → live, **≤ 10 min p95** (CI build + deploy +
  publish callback).
- Publish outcome visible as a commit status on the commit within the same window, with a
  concise error summary (statuses cap at ~140 chars; full file/line diagnostics via
  `blog check`).
- `blog check` locally validates the entire content tree in **≤ 2 s**.

**Technical:**
- TTFB: **≤ 50 ms** on cache hit, **≤ 300 ms** on cache miss (KV read + SSR render), measured
  at the edge.
- Lighthouse performance **≥ 95** on post pages.
- `site` worker binary **≤ 10 MB gzipped** (Workers Paid plan limit — hard), tracked in CI with
  an alert threshold at 5 MB; client hydrate WASM tracked, target ≤ 400 KB gzipped.
- Invariant: publishing or editing post N triggers **zero** work on any other post; deploying
  code triggers **zero** re-parsing of any post (cache purge only).
- Zero scheduled/idle compute: no cron, no containers, no Durable Objects.

## User Stories

**Author — happy paths**
1. As an author, I want to write a post in markdown with normal prose, links, lists, and code
   fences, so that writing feels like writing, not programming.
2. As an author, I want to drop a component into prose as `<Callout kind="warning">…</Callout>`
   with markdown children, so that posts can be richer than plain markdown.
3. As an author, I want to push a content-only commit to `main` and see the post live in
   seconds, so that publishing has no ceremony.
4. As an author, I want to edit a typo and push, so that the fix is live in seconds and only
   that post's cache is touched.
5. As an author, I want to delete a post's directory and push, so that the post 404s, leaves
   the index, and disappears from feeds.
6. As an author, I want frontmatter (`title`, `date`, `tags`, `draft`) to drive listings, tag
   pages, and feeds, so that metadata lives with the content.
7. As an author, I want to define a one-off interactive component in a `components.rs` file
   next to my post, so that a single post can ship custom interactive Rust without touching the
   main app.
8. As an author, I want a push containing `components.rs` to automatically build, deploy, and
   then publish the post in the right order, so that I never think about sequencing.
9. As an author, I want to add a shared component to the registry crate and have every existing
   post that references it pick up the new version with no rebuilds, so that presentation
   evolves without content churn.

**Author — drafts & feedback**
10. As an author, I want to write in-progress posts on a branch, so that nothing goes live
    until I merge.
11. As an author, I want `draft: true` frontmatter to keep a published post out of the index,
    feeds, and tag pages while remaining reachable by slug, so that I can preview on real
    devices before listing it.
12. As an author, I want a green/red commit status on my commit stating exactly which posts went
    live or why publishing failed, so that I never wonder whether a publish worked.
13. As an author, I want a typo'd component name (`<OrbitSimulatr>`) to fail the publish with a
    "did you mean" error on the commit — not render a broken page, so that errors surface at
    publish time.
14. As an author, I want to run `blog check` locally (or as a pre-commit hook), so that I catch
    unknown components, bad props, and malformed frontmatter before pushing.
15. As an author, I want `blog publish --local` as a break-glass path, so that I can publish
    from my laptop if the webhook path is ever down.

**Reader**
16. As a reader, I want post pages to load essentially instantly anywhere in the world, so that
    reading is frictionless.
17. As a reader, I want interactive widgets embedded in posts to hydrate and respond, so that
    demos are live, not screenshots.
18. As a reader, I want the page to be fully readable before/without JS, so that content is
    never held hostage by hydration.
19. As a reader, I want light/dark theming and refined typography, so that reading is
    comfortable and the blog has a distinct visual identity.
20. As a reader, I want to browse posts by tag, so that I can find related writing.
21. As a reader, I want an RSS/Atom feed and sitemap, so that I can subscribe and search
    engines can index.

**Maintainer / developer**
22. As the maintainer, I want the SSR worker to carry only the renderer, registry, and
    components (no parser, no GitHub client), so that the serving binary stays small.
23. As the maintainer, I want all publish/webhook secrets confined to the pipeline worker, so
    that the public-facing worker holds no credentials.
24. As the maintainer, I want deploys to purge the whole cache automatically, so that stale
    HTML can never reference dead hashed assets or mismatch hydration.
25. As the maintainer, I want the AST schema versioned, so that old KV entries are detectable
    and migratable when the schema evolves.
26. As the maintainer, I want parsing/validation/rendering logic in plain Rust crates testable
    with `cargo test` (no Workers runtime), so that the core is trivially testable.
27. As the maintainer, I want CI to fail if the worker binary exceeds size budgets, so that
    bloat is caught at PR time.
28. As the maintainer, I want `wrangler dev` to serve the full SSR site locally, so that I can
    develop against the real runtime.

**Edge cases & system behavior**
29. As the system, I want pushes to non-`main` branches acknowledged and ignored, so that
    branch work never publishes.
30. As the system, I want pushes touching neither `content/` nor code paths ignored, so that
    README edits are no-ops.
31. As the system, I want a mixed commit (`.mdx` + `.rs`) to take the code path with the post
    publish deferred to the CI callback, so that a post never references a component that
    isn't deployed yet.
32. As the system, I want a post referencing a component from a still-deploying *earlier*
    commit to be parked as pending and retried on the next CI callback, so that the rare
    cross-commit race self-resolves.
33. As the system, I want a KV miss on a post slug to be a plain 404 — never a trigger to
    rebuild — so that pipeline bugs surface instead of self-healing silently.
34. As the system, I want webhook requests with invalid HMAC signatures rejected, so that only
    GitHub can trigger publishes.
35. As the system, I want the publish endpoint authenticated, so that only CI (and the
    webhook path internally) can invoke it.

## Implementation Decisions

**Modules (Cargo workspace):**

| Module | Responsibility | Interface (conceptual) |
|---|---|---|
| `content-ast` | The versioned serde AST schema: document, frontmatter, node enum (`Heading`, `Paragraph`, `CodeBlock`, `List`, `Link`, `Image`, `Html`, `Component{name, props, children}`, …) *(amended in Slice 5: also `IndexEntry`, the KV `index` element — it is the other KV-schema contract, and the site worker must read it without pulling the parser)* | Types + (de)serialization + `schema_version`; no logic |
| `content-parser` | markdown-rs MDX-mode parsing → `content-ast`; frontmatter extraction; validation against the component manifest (unknown components, missing/mistyped props, rejected JS-isms) with source locations | `parse(source) -> Result<Document, Vec<Diagnostic>>`, `parse_validated(source, manifest) -> Result<Document, Vec<Diagnostic>>` *(amended in Slice 4: originally `validate(doc, manifest)`, but source positions exist only on the markdown tree — the stored AST carries none (ADR-0002) — so validation is fused into parsing to keep file/line diagnostics)* |
| `registry` | `#[post_component]` proc macro: prop conversion codegen, `inventory` registration, manifest emission; runtime dispatch `render(name, props, children) -> AnyView` | Macro + `lookup(name)` + `manifest()` |
| `app` | Leptos UI: routes, layout, AST renderer (node → view mapping), shared components (v1: `Callout` + one demo island), Tailwind v4 theme (CSS-first oklch design tokens, light/dark, Libre Baskerville / Lora / IBM Plex Mono) | `render_document(doc) -> impl IntoView` + route tree |
| `workers/site` | Worker entry: axum router, Cache API front, KV reads, RSS/sitemap/tag rendering from the index | HTTP |
| `workers/pipeline` | Webhook handling, path-based routing decision, publish op, GitHub content fetch, commit statuses, purge, pending stash | HTTP (webhook + authenticated `/publish`) |
| `publish-core` | *(added in Slice 5)* The publish operation's pure core, shared by `blog-cli` and `workers/pipeline`: parse+validate post sources against the manifest, merge the index, lay out KV writes/deletes; wasm-clean (no fs/HTTP/clock — callers own transport) | `check(sources, manifest) -> Result<Vec<ParsedPost>, Vec<Diagnostic>>`, `plan(prev_index, changed, removed) -> PublishPlan` |
| `blog-cli` | `blog check` (parse+validate content tree), `blog publish --local`/`--all` | CLI over the same crates |

**KV schema** (single namespace):
- `post:{slug}` → `{ schema_version, frontmatter, ast }`
- `index` → ordered `[{slug, title, date, tags, draft, description?}]` *(amended in Slice 10:
  optional `description`, the post's feed summary — absent entries serialize unchanged)*;
  drafts stored but filtered out of all
  listings/feeds at render time
- `pending` → list of `{slug, sha, removed}` awaiting a CI callback *(amended in Slice 6:
  `removed` marks a parked deletion, so a code push that removes a post dir drains as a KV
  delete rather than a content fetch)*

**Publish operation contract** (one implementation, two invokers):
input = commit SHA + changed/removed content paths; behavior = fetch → parse → validate →
write/delete `post:*` → rewrite `index` → purge URLs → post commit status (`blog/publish`) on
the SHA (Commit Status API — Checks API write is GitHub-App-only; see ADR-0007 amendment). Invoked by the webhook handler directly (content-only pushes) or via the authenticated
`/publish` endpoint as CI's final step (code pushes). Single-writer assumption (personal blog):
`index` rewrites are last-write-wins; accepted.

**Cache & purge:**
- `site`: `cache.match` first; miss → render → `cache.put` with `Cache-Control: max-age=604800`.
- Publish purge set (REST purge-by-URL, all colos): changed post URLs, `/`, `/posts`,
  `/rss.xml`, `/sitemap.xml`, and `/tags/{t}` for each tag on the changed posts (+ `/tags`).
- Deploy: `purge_everything` as the CI step after `wrangler deploy` (hydration correctness).

**CI (single workflow):** triggered by `workflow_dispatch` from the pipeline worker; steps:
build → size check → `wrangler deploy` (site, pipeline as needed) → `purge_everything` →
call `/publish` for the triggering commit. Secrets: Cloudflare API token in CI;
`GITHUB_WEBHOOK_SECRET` + `GITHUB_TOKEN` + publish shared secret in the pipeline worker only.

**External integrations:** GitHub (push webhooks in; contents API + commit status API + Actions
`workflow_dispatch` out), Cloudflare (KV, Cache API, REST purge, Workers Assets).

**Base scaffold:** `cargo generate cloudflare/workers-rs templates/leptos`, restructured into
the workspace; Leptos pinned to 0.8.x; cargo-leptos (with native Tailwind v4) + worker-build;
static assets served by the Workers assets layer (asset-first routing, worker never invoked for
`/pkg/*`). Known build traps to bake in from day one: getrandom `wasm_js` RUSTFLAGS in the
wrangler build command; consistent `LEPTOS_OUTPUT_NAME`; `#[worker::send]` on Env-touching
server-side helpers.

## Architectural Decisions

Full ADRs in `docs/adrs/`; summaries:

### Runtime content pipeline (no redeploy to publish)
**Decision**: Content publishes through a live worker pipeline into KV; only code rides deploys.
**Context**: The defining product property is push-to-live-in-seconds with no global rebuilds; Leptos components are AOT-compiled Rust and Workers forbids runtime WASM loading, so code and content must have different lifecycles.
**Key Drivers**: instant publish; O(changed-post) work; platform constraint on dynamic code.
**Considered Options**: 1. Live pipeline — content flows at runtime. 2. Build-time baking — every post is a CI deploy. 3. Tier 3 — publish-time rustc → per-post client WASM.
**Chosen Option**: Live pipeline, because it preserves the product's reason to exist; baking makes every typo a deploy; Tier 3 costs deploy-grade latency for a strictly worse artifact (no SSR of dynamic modules).
**Trade-offs**: Good: seconds-to-live, no rebuild cascades. Good: pipeline worker is genuinely interesting engineering. Bad: component vocabulary fixed at deploy time.
**See also**: `docs/adrs/adr-0001-runtime-content-pipeline.md`

### Versioned structured AST in KV as the content IR
**Decision**: KV stores a serde-typed semantic AST (prose nodes + component references by name), never HTML strings and never raw markdown.
**Context**: The stored artifact determines what changes force re-processing and where errors surface.
**Key Drivers**: component/prose presentation upgrades must apply to all posts with zero rebuilds; publish-time (not request-time) validation; lean SSR binary (parser excluded); hydration correctness (components render as live Leptos views).
**Considered Options**: 1. Pre-rendered HTML. 2. Structured AST. 3. Raw markdown parsed per request.
**Chosen Option**: Structured AST, because HTML freezes presentation into content (rebuild cascade returns) and likely breaks island hydration; raw markdown moves errors to request time and the parser into the hot binary.
**Trade-offs**: Good: "KV stores meaning, deployed code owns presentation." Good: `.rsx` static backend can reuse the same IR later. Bad: we own a schema and its versioning/migration story.
**See also**: `docs/adrs/adr-0002-structured-ast-ir.md`

### MDX-syntax subset; props are data, not code
**Decision**: `.mdx` files with markdown prose + PascalCase component tags parsed by markdown-rs MDX mode; literal-only props; `import`/`export`/`{expressions}` rejected at publish time.
**Context**: Real MDX is markdown+JavaScript; we can never evaluate JS, but the authoring feel and editor tooling of MDX are the point.
**Key Drivers**: MDX-grade authoring UX; zero custom parser; existing editor MDX support.
**Considered Options**: 1. MDX subset. 2. Markdown directives (`:::name`). 3. Hugo-style shortcodes.
**Chosen Option**: MDX subset, because the parser exists, editors highlight it, and publish-time validation makes the unsupported constructs loud instead of surprising.
**Trade-offs**: Good: the familiar MDX authoring feel. Bad: it looks like MDX but isn't — the subset must be documented (`CONTENT.md`). Bad: structured props limited to literals/children in v1.
**See also**: `docs/adrs/adr-0003-mdx-subset-authoring.md`

### Co-located per-post components; content/code hybrid pipeline (Tier 2)
**Decision**: Posts may ship `components.rs` beside `index.mdx`, compiled into the registry via build-time discovery; pushes containing code ride CI, content-only pushes ride the fast path.
**Context**: Posts need one-off custom components living next to the post that owns them, without polluting the shared design system; the design must reconcile "code needs a deploy" with "posts publish instantly."
**Key Drivers**: rust-analyzer only works on real `.rs` files (inline Rust-in-markdown has no LSP story); consistency of the code-deploys/content-flows principle.
**Considered Options**: 1. Registry-only (shared components, no per-post code). 2. Co-located `.rs` + hybrid pipeline. 3. Publish-time rustc containers (Tier 3).
**Chosen Option**: Co-located + hybrid, because one-off components stay next to the posts that own them with a first-class IDE story, and cost one CI cycle at authoring time, never at read time.
**Trade-offs**: Good: full type checking across posts and design system. Bad: content repo pushes can trigger deploys; CI reliability becomes part of the publish path for code-bearing posts.
**See also**: `docs/adrs/adr-0004-colocated-components-hybrid.md`

### Proc-macro component registry with emitted manifest
**Decision**: `#[post_component]` generates prop parsing, registers via `inventory`, and emits a machine-readable manifest of names/props/types.
**Context**: The registry (name + string props → typed component call) must come from somewhere; hand-written match arms vs codegen.
**Key Drivers**: DX ("just annotate the component"); the manifest enables publish validation, `blog check`, and a future `.mdx` LSP from one source of truth; `inventory` is proven on wasm (Leptos server fns).
**Considered Options**: 1. Hand-written match + manual prop parsing. 2. Attribute macro + inventory. 3. build.rs syn-based codegen.
**Chosen Option**: Attribute macro (v1 scope: scalar props + children + manifest), because it's the piece that makes this a framework rather than a script, and the manifest's three consumers justify the macro investment.
**Trade-offs**: Good: zero per-component boilerplate; introspectable vocabulary. Bad: proc-macro development/debugging cost lands in v1. Bad: macro scope creep is a real risk — v1 boundary is explicit.
**See also**: `docs/adrs/adr-0005-macro-registry-manifest.md`

### Two-worker topology split on read/write
**Decision**: `site` (SSR + KV reads) and `pipeline` (webhook + publish); no containers, Durable Objects, or separate read-API worker.
**Context**: JS/MDX pipelines need an external runtime for bundling; a Rust parser runs in-worker, so the only question is how to split responsibilities across workers.
**Key Drivers**: SSR binary leanness; secret isolation (no credentials in the public worker); independent deploy cadences; read path vs write path is the real fault line.
**Considered Options**: 1. One worker for everything. 2. Two workers. 3. Finer-grained split (separate webhook router, parser, and read-API workers).
**Chosen Option**: Two workers, because one worker puts parser+GitHub client+secrets in the serving binary, and three reintroduces a serialization hop (workers-rs service bindings are fetch-based) for isolation nobody needs.
**Trade-offs**: Good: minimal operational surface (two scripts, one KV namespace). Good: pipeline deploys don't purge the site cache. Bad: two wrangler configs/deploys to keep coherent.
**See also**: `docs/adrs/adr-0006-two-worker-topology.md`

### Single publish operation, two invokers; CI provides ordering
**Decision**: The pipeline worker owns one publish op; the webhook invokes it directly for content-only pushes, CI invokes it (after deploy) for code pushes; the worker decides the path from webhook diff paths.
**Context**: Mixed commits create an ordering problem (post referencing a not-yet-deployed component); who sequences deploy-then-publish?
**Key Drivers**: fast path must stay ~2 s; ordering correctness without a distributed state machine; observability for both paths.
**Considered Options**: 1. Worker decides + worker-managed pending/deploy tracking. 2. CI decides everything (every publish pays Actions latency). 3. No pipeline worker — CLI in CI writes KV directly.
**Chosen Option**: Hybrid of 1+2 — worker routes, CI sequences the code path and calls back, because it keeps instant content publishes while CI's step ordering replaces the state machine; the residual cross-commit race reduces to a pending-retry list. The CLI (option 3) ships anyway as break-glass and fallback posture.
**Trade-offs**: Good: both paths report to one place (the `blog/publish` commit status on the commit). Good: graceful degradation path to CLI-only. Bad: `GITHUB_TOKEN` must live in the pipeline worker for the fast path (webhooks carry paths, not contents).
**See also**: `docs/adrs/adr-0007-publish-orchestration.md`

### Cache API with targeted purge; deploys purge everything
**Decision**: Full-response caching via the per-colo Cache API; publishes purge an enumerated URL set globally via REST purge-by-URL; deploys `purge_everything`.
**Context**: Cloudflare has no cache in front of Workers (the worker always runs; hits are sub-ms early returns); purge-by-tag is Enterprise-only; cached HTML embeds hashed asset URLs + hydration markup coupled to the deployed binary.
**Key Drivers**: static-site-grade hit performance; publishing post N must not touch post M's cache; post-deploy correctness (dead `/pkg/*` refs, hydration mismatch).
**Considered Options**: 1. No caching. 2. Cache API + URL purge + deploy nuke. 3. Build-ID cache-key generations (purge-free deploys).
**Chosen Option**: Option 2, because option 1 leaves easy performance unclaimed and option 3 requires the pipeline worker to discover the site's build ID — machinery unjustified at this scale (kept as upgrade path).
**Trade-offs**: Good: hit path does no KV read and no render. Good: 7-day TTL backstop self-heals missed purges. Bad: per-colo cold renders after deploys; one more API credential (purge) in CI and pipeline.
**See also**: `docs/adrs/adr-0008-cache-and-purge.md`

## Testing Decisions

- **What makes a good test**: test external behavior and contracts between modules, not
  internals. The architecture concentrates logic in plain Rust crates precisely so the
  interesting behavior is testable with `cargo test` on the native target — no Workers runtime,
  no mocks of Cloudflare APIs for the core.
- **Modules tested**:
  - `content-parser` (highest value): golden tests — fixture `.mdx` files → expected AST JSON;
    diagnostic tests — unknown component, missing/mistyped prop, `import`/expression rejection,
    malformed frontmatter, each asserting message + source location. The fixture corpus doubles
    as the authoring-format spec.
  - `content-ast`: serde round-trip and schema-version compatibility (old-version fixture must
    still deserialize or fail detectably).
  - `app` renderer: AST → `leptos::ssr::render_to_string` snapshot tests per node type,
    including component dispatch and children recursion.
  - `registry` macro: manifest emission correctness for representative signatures; compile-fail
    cases (unsupported prop type) via trybuild if cheap, else deferred.
  - `workers/pipeline`: pure decision logic (path classification, purge-set computation,
    pending handling) extracted into testable functions; the worker shim stays thin and is
    exercised by `wrangler dev` smoke tests, not unit tests.
  - `blog-cli`: integration test over a fixture content tree (check passes/fails as expected).
- **Prior art**: Workers-runtime test harnesses are high-friction and tend to rot into unused
  scaffolding; this design avoids depending on them — thin worker shims, fat natively-testable
  crates.

## Out of Scope (v1)

- **`.rsx` static backend** / `include_post!` (baked pages) — v2 side-project; protected by the
  shared AST IR.
- **`.mdx` LSP** (diagnostics/autocomplete) — v2; enabled by the manifest, not built yet.
- **Tier 3** publish-time-compiled per-post client WASM.
- **CodeBlock island** (client-side syntax highlighting with theme picker) — v1 renders code
  fences as plain `<pre><code>` with the theme's styling; the island is the first post-v1
  vocabulary addition.
- **Figure/Image component and image asset hosting** — v1 posts are text + components; where
  images live (repo vs R2) is deliberately unresolved.
- **Branch deploy previews**; v1 preview = `blog check` + local `cargo leptos watch`.
- **`/status` page** and `publish:log` journal.
- **About page** as content; if wanted in v1 it's a hardcoded route.
- Comments, search, analytics, newsletter — not this project.
- Build-ID cache generations (documented upgrade path only).
- Multi-author/multi-repo concerns; single-writer `index` is accepted.

## Further Notes

- **Sequencing** (tracer-bullet order agreed in the design session): (1) scaffold from the
  workers-rs Leptos template, SSR + hydration working under `wrangler dev` and deployed once;
  (2) `content-ast` + `content-parser` with the fixture corpus; (3) AST renderer in `app` with
  a hand-wired component before the macro exists; (4) macro + manifest; (5) pipeline worker +
  publish op + commit statuses; (6) CI workflow + purge; (7) RSS/sitemap/tags; (8) visual theme.
- **Dependencies**: Cloudflare Workers Paid plan ($5/mo — 10 MB limit accepted as the budget);
  a fine-grained GitHub PAT with contents:read + commit-statuses:write + actions:write
  permissions (checks:write is unusable — the Checks API is GitHub-App-only, hence commit
  statuses; see ADR-0007 amendment); webhook + publish shared secrets provisioned via
  `wrangler secret`.
- **Ecosystem risk, accepted**: Leptos is feature-complete but "lightly maintained" (May 2026
  maintainer statement); pinned to 0.8.x with no expectation of 0.9.
- **Importing existing content**: posts already written in the `content/blog/{slug}/index.mdx`
  shape are onboarded by running the publish operation over the tree (`blog publish --all`);
  there is no storage-level migration — KV is populated exclusively through the pipeline.
