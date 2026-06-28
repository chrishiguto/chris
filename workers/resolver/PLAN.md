# Resolver Worker Transformation Plan

## Objective
Transform the resolver worker (copied from orchestrator) into an RPC-enabled worker that serves as a middleware between `apps/web` and the Cloudflare `POSTS_CACHE` KV namespace.

## Changes Required

### 1. Update `workers/resolver/wrangler.jsonc`
- Change worker name from "orchestrator" to "resolver"
- Remove the `BUNDLER` service binding (not needed for resolver)
- Add `POSTS_CACHE` KV namespace binding (same as bundler worker uses):
  ```jsonc
  "kv_namespaces": [
    {
      "binding": "POSTS_CACHE",
      "id": "2faa8539d8334ddfb507c2f9191b52a0"
    }
  ]
  ```
- Keep compatibility flags: `nodejs_compat`, `nodejs_compat_do_not_populate_process_env`
- Remove the `services` array

### 2. Rewrite `workers/resolver/src/index.ts`
- Replace `ExportedHandler` pattern with `WorkerEntrypoint` class (for RPC support)
- Import `WorkerEntrypoint` from `cloudflare:workers`
- Define `Env` interface with:
  ```typescript
  export interface Env {
    POSTS_CACHE: KVNamespace;
  }
  ```
- Export default class extending `WorkerEntrypoint<Env>`
- Implement two RPC methods as stubs:
  - `getPosts()`: returns placeholder empty array `[]`
  - `getPost(id: string)`: returns placeholder `null`
- Remove all webhook-related code:
  - Signature verification
  - Push event validation
  - GitHub webhook parsing
  - Service binding calls to BUNDLER
- Remove imports from `@octokit/webhooks` and `@repo/schemas`

### 3. Update `workers/resolver/package.json`
- Remove unnecessary dependencies:
  - `@octokit/webhooks`
  - `@octokit/webhooks-types`
  - `@repo/schemas`
  - `esbuild`
- Keep only essential dev dependencies:
  - `typescript`
  - `vitest`
  - `wrangler`
  - `@cloudflare/vitest-pool-workers`

### 4. Delete unused files
- Remove `workers/resolver/src/webhooks.ts` (if it exists, copied from orchestrator)

## Expected Result

A clean RPC-enabled Cloudflare Worker that:
- Can be called via service bindings from `apps/web`
- Provides two RPC methods: `getPosts()` and `getPost(id: string)`
- Has access to the `POSTS_CACHE` KV namespace
- Contains only stub implementations (no actual logic yet)
- Is ready for future implementation of KV read operations

## Example Usage Pattern (Future)

```typescript
// In apps/web
const resolver = env.RESOLVER; // Service binding
const posts = await resolver.getPosts();
const post = await resolver.getPost('hello-world');
```