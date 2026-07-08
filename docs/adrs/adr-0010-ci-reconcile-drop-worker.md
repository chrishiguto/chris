# ADR-0010: CI-side reconcile — drop the pipeline worker and coordinator DO

**Status**: Proposed (2026-07-08) — **not implemented**. Captured for later evaluation as the
next simplification after ADR-0009's 2026-07-08 amendment (the "Architecture 2" we deferred).
**Related**: supersedes, if accepted, ADR-0006 (two-worker topology) and the write-path half of
ADR-0009; depends on the same `publish`/`xtask` crates.

## Context

ADR-0009's 2026-07-08 amendment moved every publish onto one GitHub Actions workflow that calls
the pipeline worker's synchronous `/publish`. That collapsed the two trigger mechanisms into one
and made the Actions run the observability surface. But it left the pipeline worker + coordinator
Durable Object in place, and their remaining jobs are now individually redundant with things CI
already has:

- **Serialization** — the workflow's `concurrency: { group: publish, cancel-in-progress: false }`
  already runs at most one publish at a time.
- **Content fetch** — the reconcile fetches every post source from the GitHub contents API, but
  the runner has the whole tree checked out already.
- **Snapshot plan + write + pointer flip** — `xtask plan` + `wrangler kv bulk put` + a `current`
  flip already do exactly this for the `just publish` break-glass path.
- **Scoped purge** — a diff of the previous index against the new one; the data (previous index,
  `current` pointer) is readable from KV via `wrangler kv key get`.
- **Retention sweep** — list `snapshot:*` keys and delete beyond the last 10.

In other words, once the fast path is gone, the worker + DO are a hop that CI can do inline. This
is ADR-0009's **Option 3** ("reconcile-to-HEAD in CI"), which was rejected there for one reason
only — "its serialization (Actions concurrency group) only works because every refresh rides CI,
which would forfeit the ~2 s fast path." That objection no longer holds: every refresh now rides
CI by design.

## Proposal

Delete `workers/pipeline` (lib/server/net/coordinator), its Durable Object + migration, and its
`wrangler.toml`. The `publish` job in `.github/workflows/publish.yml` reconciles directly:

1. (code path unchanged) build → size gate → deploy site → deploy nothing-called-pipeline.
2. `xtask plan --sha <HEAD>` builds the immutable snapshot from the checkout (no API fetch;
   validation runs against the checkout's own compiled manifest — the same commit just deployed).
3. `wrangler kv bulk put` the snapshot keys, then flip `current` (atomic from the reader's side,
   exactly as today).
4. Compute the scoped purge set: `wrangler kv key get` the old `current` + its index, diff via
   `publish::stale_tags`, and `POST /__purge` to the site (over HTTPS with `PURGE_SHARED_SECRET`,
   as `just purge` already does — no `SITE` service binding needed).
5. Sweep retention: list `snapshot:*`, delete beyond the last 10.
6. Emit the outcome (`published`/`failed`/`purged`) to the run log; a non-`ok` outcome fails the
   job, reddening the `environment: content` deployment on the merged PR (unchanged from Arch 1).

Serialization is the Actions concurrency group. Two rapid merges queue; GitHub keeps the latest
pending and cancels older pending runs, so the tree converges to HEAD — over-reconciling is safe
by construction (ADR-0009), so this is acceptable.

## What this deletes

- The entire `workers/pipeline` crate, its build (`just build-pipeline`), and its `wrangler.toml`.
- The coordinator Durable Object and its migration (one less stateful primitive; ADR-0006 returns
  to a genuinely two-*script* system, or one script + CI).
- The fine-grained GitHub PAT (`GITHUB_TOKEN`) entirely — CI reads content from its checkout, not
  the API.
- `PUBLISH_SHARED_SECRET` and the `/publish` endpoint (no worker to authenticate to).
- The `SITE` service binding (purge goes direct to the site's public `/__purge`).
- `authn` shrinks to what the site's `/__purge` needs (`verify_bearer`).

`just publish` (break-glass) and the primary path converge into essentially the same mechanism —
the escape hatch becomes the main road, which is a simplification in itself.

## Tradeoffs

- **For**: a whole worker + DO + PAT + shared secret + second config deleted; the reconcile logic
  already exists in `xtask`/`publish`; validation against the checkout's own manifest removes tens
  of contents-API round-trips per publish; one fewer deploy target.
- **Against**: reverses ADR-0009's central "one DO serializes every mutation" decision —
  serialization now leans on Actions concurrency, whose ordering is "latest pending wins" rather
  than strict FIFO (fine here, since reconcile-to-HEAD converges regardless). KV writes happen
  over the network from the runner rather than from a worker colocated with KV (more wall-clock,
  same atomicity). Purge debt (a failed purge's tags, retried next reconcile) currently lives in
  DO storage; without the DO it must move to a KV key or be recomputed from the index diff — a
  small amount of new `xtask` state to design.

## Open questions

- Where does purge debt live without the DO? Candidate: a `purge-debt` KV key the next reconcile
  reads and merges, mirroring the DO storage key it replaces.
- Are `wrangler kv bulk put` writes from CI atomic enough for the snapshot-then-flip contract?
  (They are today for `just publish`, which uses the identical sequence.)
- Is the extra runner wall-clock (network KV writes + a plan build) acceptable versus Arch 1's
  in-worker reconcile? Likely yes at blog scale; measure before committing.

## Recommendation

Not now. Architecture 1 (implemented) already delivers the observability goal with lower risk and
keeps the DO's clean serialization. Revisit this ADR if maintaining the pipeline worker + DO
proves to be ongoing overhead, or if a future change would otherwise touch the worker's transport
layer anyway — at which point deleting it outright is cheaper than maintaining it.
