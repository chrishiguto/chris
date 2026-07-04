# Publishing with the `blog` CLI

The `blog` CLI is the v1 publish path and the permanent break-glass path
(ADR-0007): validate the content tree locally, then write posts and the
listing index straight to KV through the Cloudflare API. The webhook fast
path (pipeline worker, Slice 6) reuses the same `publish-core` logic.

## `blog check`

```sh
cargo run -p blog-cli -- check
```

Parses and validates every `content/blog/{slug}/index.mdx` against the
component manifest collected from the compiled `app` crate — exactly the
vocabulary the deployed site renders with. Any problem (unknown component,
bad prop, malformed frontmatter, non-ISO date, a post directory without
`index.mdx`) prints as `file:line:column: message` and exits non-zero, so it
works as a pre-commit hook.

## One-time setup for `blog publish --local`

KV writes go through the Cloudflare REST API with a scoped token — never a
global API key.

1. **Namespace id**: `npx wrangler kv namespace create BLOG` (once per
   account); paste the id into `wrangler.toml` and keep it handy.
2. **Scoped token**: Cloudflare dashboard → My Profile → API Tokens →
   Create Token → Custom. Grant exactly one permission:
   **Account → Workers KV Storage → Edit**, scoped to your account.
3. **Environment** (e.g. in your shell profile or an untracked `.env`):

```sh
export CLOUDFLARE_ACCOUNT_ID=…   # dashboard → Workers & Pages → account id
export BLOG_KV_NAMESPACE_ID=…    # from step 1
export CLOUDFLARE_API_TOKEN=…    # from step 2
```

## Publishing

```sh
# one post (break-glass: only this post has to be valid)
cargo run -p blog-cli -- publish --local components-demo

# the whole tree: publishes every post, rewrites the index from the tree,
# and deletes KV posts whose directories no longer exist locally
cargo run -p blog-cli -- publish --local --all
```

Both write `post:{slug}` documents plus the rewritten `index` (newest-first)
in one bulk call, so `/`, `/posts`, and `/posts/{slug}` are live immediately
after the command returns. `--all` is also the onboarding path for content
written before the pipeline existed (PRD "Importing existing content").

Note: until Slice 8 (cache layer) lands, pages are rendered per-request, so
there is nothing to purge after a publish.
