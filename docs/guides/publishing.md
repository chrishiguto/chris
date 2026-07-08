# Manual publishing with `just` + wrangler

The `just publish` recipe is the permanent break-glass path (ADR-0007/0009)
and the bulk-import path: validate the content tree locally, plan one
immutable snapshot with `xtask`, and move the bytes with wrangler — the same
`publish` crate logic the pipeline worker's reconcile runs. It deliberately
bypasses the coordinator Durable Object (it is the escape hatch for when the
pipeline worker itself is the problem); the next reconcile supersedes
whatever it wrote by rebuilding from HEAD. There is no dedicated CLI and no
extra API token: auth is wrangler's own (your local `wrangler login` session,
or `CLOUDFLARE_API_TOKEN` in automation — the same credentials deploys use).

## `just check`

```sh
just check          # fmt + clippy + content-tree validation
cargo run -p xtask -- check   # just the content check
```

Parses and validates every `content/blog/{slug}/index.mdx` against the
component manifest collected from the compiled `app` crate — exactly the
vocabulary the deployed site renders with. Any problem (unknown component,
bad prop, malformed frontmatter, non-ISO date, a post directory without
`index.mdx`) prints as `file:line:column: message` and exits non-zero, so it
works as a pre-commit hook.

## Publishing

```sh
# the whole tree as one snapshot: publishes every post, rebuilds the index,
# retires posts whose directories no longer exist locally
just publish

# against the local `wrangler dev` simulator (e2e testing)
just remote='--local' publish
```

A snapshot is by definition complete, so there is no per-slug mode anymore:
the whole local tree must validate (a broken draft blocks the publish — fix
or remove it locally first). Under the hood the recipe is three steps plus a
purge, all inspectable in the justfile: plan
(`xtask plan --sha manual-… ` → `target/publish/{writes,pointer}.json`),
apply the snapshot keys (`wrangler kv bulk put`), flip the pointer
(`wrangler kv key put current --path`). The pointer flips only after
every snapshot key landed, so readers see the whole old snapshot or the whole
new one — never a blend. This is also the onboarding path for content written
before the pipeline existed (PRD "Importing existing content"), and the
**rollback** mechanism: put an older retained snapshot's sha back into
`current` (see `pipeline-deploy.md` → Operations).

## Cache purge

Pages are cached by Workers Cache (ADR-0008 amendment), which only the site
worker itself can purge — no zone API, no wrangler command can reach it. The
recipe's last step therefore runs `just purge`, which curls the site's
authenticated `/__purge` route (the same one the pipeline's reconcile and
CI's post-deploy step call) with `{"tags":["site"]}` — a manual publish
bypasses the coordinator's changed-post diff, so it evicts everything:

```sh
export SITE_ORIGIN=…             # e.g. https://chris-site.<subdomain>.workers.dev
# .secrets/purge_shared_secret must hold the workers' PURGE_SHARED_SECRET
```

With either missing the purge is skipped with a note, and cached pages
converge via the 7-day `s-maxage` backstop or the next CI deploy's `site`
purge. Nothing here can fail the already-applied publish. `just purge` also
works standalone as the break-glass "evict everything now".

## One-time setup

1. **Namespace**: `npx wrangler kv namespace create BLOG` (once per account);
   paste the id into `wrangler.toml`.
2. **Auth**: `npx wrangler login` locally — that's it. In CI or scripts, set
   `CLOUDFLARE_ACCOUNT_ID` + `CLOUDFLARE_API_TOKEN` (the token needs
   **Workers KV Storage: Edit**; the deploy token can simply gain that scope).
