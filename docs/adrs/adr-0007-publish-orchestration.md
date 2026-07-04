# ADR-0007: Single publish operation, two invokers; CI provides ordering

**Status**: Accepted (2026-07-03); amended 2026-07-04 — Check Runs replaced by commit statuses (see note in Decision)
**Related**: PRD `docs/prds/prd-leptos-workers-blog-v1.md`; depends on ADR-0004, ADR-0006

## Context

A push may contain content, code, or both. Code must deploy before content referencing it
publishes (ADR-0004). Someone must (a) decide which path a push takes and (b) guarantee
deploy-before-publish ordering for mixed commits — ideally without a distributed state machine.
Also: publishes need observability regardless of path.

## Decision

The **pipeline worker decides**, by inspecting the webhook payload's `added/modified/removed`
paths (no SHAs, no CI involvement in the decision). One **publish operation** (fetch → parse →
validate → KV → purge → commit status) with two invokers:

- **Content-only push** → webhook fast path: the worker publishes directly. Live in ~2 s;
  CI never runs.
- **Push containing `.rs`/app code** → the worker stashes the pending post list in KV, fires
  `workflow_dispatch`, and stops. One CI workflow: build → deploy → call the authenticated
  `/publish` endpoint → pendings drained. **Ordering is guaranteed by CI's step sequence**, not
  by worker state tracking.

Residual cross-commit race (post referencing a component whose deploy from an *earlier* commit
is still in flight) is caught by publish-time validation → parked in `pending` → retried on the
next CI callback. Observability: the worker posts a **GitHub commit status** (context
`blog/publish`) on the pushed SHA for *both* paths — the commit page is the publish dashboard.

> **Amendment (2026-07-04)**: originally specified as a GitHub **Check Run**, but write access
> to the Checks API is GitHub-App-only — no PAT of any kind can create one. Rather than run a
> GitHub App (app registration, private-key storage, JWT/installation-token code in the
> worker), v1 uses the **Commit Status API**, which works with a fine-grained PAT
> (commit statuses: read/write). Trade-off: a status carries only a ~140-char description and
> a target URL — no rich markdown output panel — so the status holds a concise summary and
> full file/line diagnostics remain the job of `blog check`. Revisit as a GitHub App if v2
> wants inline annotations.

A `blog publish --local` CLI ships as break-glass; if the webhook path is ever retired, the
CLI-in-CI posture (option 3 below) is reachable without redesign.

## Options considered

1. **Worker decides + worker tracks deploys/pendings** — the fast path everywhere, but mixed
   commits require real distributed-systems work (deploy-completion tracking, retries).
2. **CI decides everything** — ordering free, no GitHub token in workers, but every publish
   (even a typo fix) pays ~30–60 s of Actions latency.
3. **No pipeline worker** — CLI in CI parses natively, writes KV via Cloudflare API. Simplest;
   loses the fast path and the worker engineering entirely.
4. **Hybrid of 1+2** — chosen: worker routes, CI sequences the code path and calls back.

## Consequences

- Good: content publishes stay instant; ordering correctness costs a two-line pending list.
- Good: one publish implementation; both paths converge on the `blog/publish` commit status
  for observability.
- Bad: `GITHUB_TOKEN` must live in the pipeline worker (webhook payloads carry paths, not file
  contents, so the worker must fetch content itself).
- Bad: the code path depends on GitHub Actions availability (accepted; break-glass CLI exists).
