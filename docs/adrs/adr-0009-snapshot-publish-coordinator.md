# ADR-0009: Immutable content snapshots, reconcile-to-HEAD, one coordinator

**Status**: Accepted (2026-07-07)
**Related**: PRD `docs/prds/prd-leptos-workers-blog-v1.md`; supersedes the ordering mechanism of ADR-0007 (its decision is amended, not retired); amends ADR-0008's purge derivation

## Context

ADR-0007 claimed "ordering correctness costs a two-line pending list". It didn't. The
pending list was shared mutable state with two concurrent writers (webhook, CI callback) on
a store with no compare-and-swap, and every publish was a *delta* pinned to its push's SHA.
Two consequences surfaced in review:

- **Stale-delta revert**: a code push parks post X at `shaA` and dispatches CI; a follow-up
  content-only typo fix publishes X at `shaB` through the fast path; A's `/publish` callback
  then drains the parked entry and rewrites X *and the index* with the older content — then
  purges, making the revert visible immediately. A quick fix during a CI build is a natural
  authoring pattern, so the minutes-long window is realistic.
- **Read-modify-write races**: `pending` and `index` were get→merge→put on last-write-wins
  KV; an overlapping park and drain could erase a parked push or drop an index entry.

Any delta-based design with retries, concurrency, or out-of-order webhook delivery (GitHub
guarantees none of these away) eventually applies a stale delta. Patching supersession into
the fast path narrows the window; it does not remove the class of bug.

## Decision

Three moves, each removing a failure class rather than shrinking it:

1. **Publishes are immutable snapshots behind one mutable pointer.** Every publish writes a
   complete content set under `snapshot:{sha}:post:*` + `snapshot:{sha}:index` and then
   flips the single `current` key (`{"sha": …}`). Readers resolve the pointer first, so a
   publish is atomic from the read side: the whole old snapshot or the whole new one, never
   a torn index/post blend. Rollback is re-pointing `current` at a retained snapshot.

2. **The publish operation is reconcile-to-HEAD, not apply-this-push.** Any trigger means
   "make KV match the branch HEAD observed now": resolve HEAD (one API call), list the post
   tree at that commit, fetch and validate every source, snapshot, flip, purge, post one
   `blog/publish` status on that HEAD. Ordering stops mattering — a late CI callback, a
   duplicate delivery, an out-of-order webhook, or a force push all converge to current
   HEAD; over-reconciling is always safe. A post that fails validation rides in as its
   previously published payload (carry-forward) or stays out if it never published, so one
   broken post never wedges the rest — and the post-deploy trigger retries it against the
   freshly deployed manifest for free, which is how deploy-before-publish is now enforced
   (validation gate, not bookkeeping).

3. **One Durable Object serializes every mutation.** A `PublishCoordinator` DO (single
   instance, SQLite-backed) owns the write path. Triggers only persist the target repo +
   branch, set a dirty flag, and pull the DO's alarm to now — they return immediately. The
   alarm handler is the sole reconcile executor: alarms never overlap and auto-retry on
   failure, triggers landing mid-run coalesce into exactly one follow-up run, and at the end
   of each run the alarm re-arms as a cron backstop (6 h), making the loop self-healing
   after missed webhooks or exhausted retries. The `pending` key, supersession logic, drain
   reports, and per-SHA status fan-out are deleted rather than fixed.

Retention: the DO keeps the last 10 flipped snapshots (rollback depth) and sweeps KV for
snapshot keys belonging to neither that history nor whatever `current` points at — the
pointer re-read protects a concurrent break-glass flip.

Break-glass (`just publish`) keeps writing KV directly through wrangler — deliberately
bypassing the coordinator, because it is the escape hatch for when the pipeline worker
itself is the problem. It writes a full `manual-*` snapshot and flips the pointer; the next
reconcile supersedes it by construction.

*Amendment (2026-07-07, Workers Cache purge):* the zone purge-by-URL step is replaced by
one call after the pointer flip: the coordinator POSTs to the site worker's authenticated
`/__purge` route over a `SITE` service binding, and the site — the only party that can
reach its own Workers Cache — runs `cache.purge({purgeEverything: true})`, global via
Instant Purge (ADR-0008's amendment has the platform context). Purge-everything is
semantically what the old enumerated set approximated: any flip invalidates every page
(the site-wide snapshot-sha ETag says as much), so the purge-set planning — `SnapshotPlan
::purge`, origin-prefixing, 30-file chunking, and the previous-index read that fed only it
— is deleted outright; `publish::snapshot` no longer takes the previous index, and
`xtask plan` no longer reads a pointer or index (`just publish` shrank to plan → bulk put
→ flip → one authenticated curl). Auth is a shared secret (`PURGE_SHARED_SECRET`, held by
both workers, checked constant-time by the shared `authn` crate — the same gate as
`/publish`; this header-secret + constant-time-compare pattern, and the webhook's HMAC
verification, follow Cloudflare's own Worker auth examples:
[Sign requests](https://developers.cloudflare.com/workers/examples/signing-requests/) and
GitHub's [webhook validation](https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries)).
It is an authenticated HTTP POST rather than a typed RPC call because of how Workers Cache
scopes purges: purge is scoped to the entrypoint that owns the cached responses, and this
site caches on its *default* public fetch entrypoint, so the purge must run there.
Cloudflare's secret-free alternative is to move caching into a private named
`WorkerEntrypoint` (their `CachedBackend` reference pattern), reachable only over the service
binding, and expose `purge()` as an RPC method on it — but workers-rs cannot author named
`WorkerEntrypoint` classes, so that restructure is unavailable. (A workers-rs capability gap,
not a consequence of the `=0.8.3` pin: experimental wasm-bindgen RPC has existed since 0.6.7,
but only on the public default entrypoint, so it would not remove the gate either.) `fetch`
over the service binding to the gated public route is therefore the transport, and the secret
gate is load-bearing, not optional. Purge stays
best-effort after the flip: a failure logs loudly and the 7-day `s-maxage` TTL (or the
next deploy's fresh version-keyed cache) backstops it. The zone purge credentials
(`CLOUDFLARE_ZONE_ID`, `SITE_ORIGIN`, `CLOUDFLARE_PURGE_TOKEN`) leave the pipeline
entirely; a custom domain is no longer a purge prerequisite.
*(Superseded by the 2026-07-09 amendment below: the `SITE`-binding purge was a no-op
— `cache.purge()` runs in the caller's entrypoint — so the purge moves to CI over HTTP
and this in-worker call, its binding, and its secret are deleted.)*

*Amendment (2026-07-08, single Actions trigger + synchronous reconcile):* the
dual trigger — a GitHub webhook to `POST /webhook` (HMAC-verified, push
classified into a content fast path vs. a `workflow_dispatch` code path) plus
CI's `POST /publish` callback — is replaced by one workflow,
`.github/workflows/publish.yml`, on `push: [main]` and `pull_request`. A merge
to main runs a single job (`environment: content`) that deploys the workers
when a paths filter sees code and then always calls `/publish`; a PR runs a
`check` job (`just check` + `just test`) visible in the PR before merge.
`/publish` now runs the reconcile **synchronously** and returns a
`PublishOutcome` (`published`/`failed`/`carried`/`purged`/`ok`/`summary`) — an
Actions step has no ~10 s webhook delivery window to protect, so the
alarm/dirty/coalesce/backstop machinery, the `blog/publish` commit-status
fan-out, and the hand-rolled GitHub Deployments API calls are all deleted. The
coordinator DO is unchanged in what it *does* — single-instance serialization,
immutable snapshot + `current` flip, scoped `/__purge`, retention — but it no
longer talks to GitHub: it returns the outcome, and the run fails (reddening
the deployment GitHub natively records on the merged PR, which links to the
run) whenever `ok` is false. Observability moves entirely onto GitHub's own
primitives: the pre-merge check and the post-merge deployment-to-run link;
there is no preview URL. This deliberately forfeits the ~2 s content fast path
that Option 3 was rejected for — that objection only held while content
publishes bypassed CI; once one visible workflow covers both paths, a second
trigger mechanism and an invisible-on-the-PR status are not worth the seconds.
Secrets shrink: `GITHUB_WEBHOOK_SECRET` is retired and the fine-grained PAT
drops to Contents RO (no `workflow_dispatch` Actions scope, no Deployments
scope).

*Amendment (2026-07-09, the coordinator stops purging):* the post-flip
`/__purge` call over the `SITE` service binding was a silent no-op —
`cache.purge()` is scoped to the entrypoint that runs it, and over a binding
that is the pipeline's, not the site's, so content-only merges went
green-while-stale (ADR-0008's 2026-07-09 amendment has the platform detail).
The coordinator no longer purges. It computes the stale scope
(`publish::stale_tags` diffing the previous index against the new one) and
returns it as `PublishOutcome.tags`; the Actions Publish step evicts those
tags by calling the site's public `/__purge` over HTTP — the only entrypoint
that can. Deleted with the in-worker purge: `net::purge_site`, the `SITE`
binding, the `PURGE_SHARED_SECRET` the pipeline held, the `pending-purge` debt
ledger and its escalation, and `purge_scope`. `PublishOutcome` drops `purged`
(so `ok` reflects validation only) and keeps `tags`. The purge-debt invariant
retires with the ledger: CI retries a transient failure and reddens the run on
a hard one, and a same-HEAD re-run re-derives the identical scope from git, so
there is nothing to carry across runs. What the DO still owns is unchanged:
single-instance serialization, the immutable snapshot + `current` flip, and
retention.

## Costs accepted

- One extra KV read per cache miss on the site (pointer resolution) — invisible behind the
  Cache API front.
- A reconcile re-fetches every post source at HEAD (tens of small contents-API calls at
  this scale) instead of just the changed ones. Chosen over diff/copy-forward machinery
  because it also re-validates old posts against the current manifest, catching component
  regressions. Revisit with per-post source hashes in the index if the tree grows painful.
- The purge set is the whole enumerated URL surface of the previous and new indexes (a full
  rebuild can't know which post bodies changed). Chunked to the API's 30-file cap; fine at
  blog scale (ADR-0008 amended). *Superseded by the 2026-07-07 amendment below: the purge
  set no longer exists.*
- Commit statuses report per reconciled HEAD, not per parked SHA — a superseded
  intermediate commit may keep a stale `pending` status. Accepted fidelity loss.
  *(Superseded by the 2026-07-08 amendment: commit statuses are removed; the
  Actions run and the `content` deployment it records on the merged PR are the
  report.)*
- Snapshots duplicate content per publish; retention bounds it at ~10 × content size.

## Options considered

1. **Patch supersession into the fast path** (remove published slugs from `pending`) —
   fixes the reported case, leaves the drain-in-flight and RMW races (no CAS in KV);
   rejected as a workaround.
2. **Serialize the existing delta design behind a DO** — closes the concurrency races but
   keeps delta semantics, so out-of-order webhook delivery still needs head-tracking state;
   rejected as building the state machine without removing the reason it's fragile.
3. **Reconcile-to-HEAD, in place (kentcdodds.com's shape)** — convergent like this design,
   but in-place cache refresh is non-atomic (readers can see mixed index/post state) and
   there is no rollback or preview; its serialization (Actions concurrency group) also only
   works because every refresh rides CI, which would forfeit the ~2 s fast path.
4. **Snapshots + pointer + coordinator DO** — chosen: convergent *and* atomic, with
   rollback and bounded history; the only serialized operation is cheap.
5. **D1 transactions / bake content into the deploy / Queues** — respectively: lateral
   (fixes CAS, no atomic-publish/rollback win, trades read topology), ADR-0001's rejected
   premise (every typo pays a Rust build), and no strict FIFO so ordering survives;
   all rejected.

## Consequences

- Good: the revert and RMW findings become unrepresentable, not unlikely; publishes are
  atomic, reversible values; the system self-heals on the next trigger or backstop tick.
- Good: the pipeline worker's decision surface shrank (pending machinery deleted); the
  reconcile is one function with pure, natively-tested vocabulary around it.
- Good: the fast path survives — content-only pushes still publish in seconds, now via a
  trigger that returns before any GitHub fetch. *(Superseded by the 2026-07-08
  amendment: content now publishes through the same Actions workflow as code,
  trading the fast path for one visible trigger and PR-native observability.)*
- Bad: a Durable Object joins the topology (ADR-0006 amended by this ADR's existence —
  the write path now has one; the read path still never touches it) and its migration
  rides `workers/pipeline/wrangler.toml`.
- Bad: workers-rs's `#[durable_object]` macro generates glue referencing bare
  `wasm_bindgen::` paths, so the pipeline crate needs a direct `wasm-bindgen` dependency
  (noted in its Cargo.toml).
- Neutral: legacy flat keys (`index`, `post:*`, `pending`) become dead the moment the first
  reconcile flips `current`; the site reader falls back to them until then, and cleanup is
  a documented one-time operation (`docs/guides/pipeline-deploy.md`).
