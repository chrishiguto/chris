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
or remove it locally first). Under the hood the recipe is five steps, all
inspectable in the justfile: read the `current` pointer and the previous
snapshot's index (`wrangler kv key get`; both feed only the purge set), plan
(`xtask plan --sha manual-… ` → `target/publish/{writes,pointer,purge}.json`),
apply the snapshot keys (`wrangler kv bulk put`), flip the pointer
(`wrangler kv key put current --path`), purge. The pointer flips only after
every snapshot key landed, so readers see the whole old snapshot or the whole
new one — never a blend. This is also the onboarding path for content written
before the pipeline existed (PRD "Importing existing content"), and the
**rollback** mechanism: put an older retained snapshot's sha back into
`current` and purge (see `pipeline-deploy.md` → Operations).

## Cache purge

After applying the plan, the recipe purges exactly the plan's URLs (the same
set the pipeline worker purges, ADR-0008) — but only when a zone exists:

```sh
export CLOUDFLARE_ZONE_ID=…      # the custom domain's zone
export SITE_ORIGIN=…             # e.g. https://blog.example.com
export CLOUDFLARE_API_TOKEN=…    # needs Zone → Cache Purge
```

With any of these unset the purge is skipped with a note — correct on
`*.workers.dev`, where the Cache API is inert and there is nothing to purge.
A failed purge never fails the already-applied publish; the site's 7-day TTL
is the backstop.

## One-time setup

1. **Namespace**: `npx wrangler kv namespace create BLOG` (once per account);
   paste the id into `wrangler.toml`.
2. **Auth**: `npx wrangler login` locally — that's it. In CI or scripts, set
   `CLOUDFLARE_ACCOUNT_ID` + `CLOUDFLARE_API_TOKEN` (the token needs
   **Workers KV Storage: Edit**; the deploy token can simply gain that scope).
