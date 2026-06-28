# Implementation Plan: Migrate Bundler Worker to Pure RPC

## Overview

This plan migrates the bundler worker from HTTP-based fetch handler to pure RPC with TWO methods:

1. **`bundlePosts(PostChange[])`** - Process specific posts (webhook-driven)
2. **`bundleAllPosts(owner, repo)`** - Process all posts (full refresh)

### Goals:

1. Convert bundler to pure RPC using `WorkerEntrypoint` class
2. Replace `fetch()` handler with `bundlePosts()` RPC method (**breaking change**)
3. Add `bundleAllPosts()` RPC method (new functionality for resolver)
4. Extract shared bundling logic to `processPosts()` (90% code reuse)
5. Update orchestrator to call `bundlePosts()` via RPC (required change)

## Current State Analysis

### Existing Flow (Webhook-driven)

The current `fetch()` handler in `/workers/bundler/src/index.ts`:

- Receives an array of `PostChange` objects with `{postName, ownerName, repoName}`
- For each post, calls `collectPostFiles()` to fetch files
- Sends files to the container for compilation
- Stores compiled output in KV at `mdx:post:{postName}`
- **Missing**: Does not update the metadata cache at `blog:posts:all`

### Key Components

- **`getOctokit()`** - Creates/caches Octokit client instances
- **`collectPostFiles()`** - Recursively fetches post files from a specific folder
- **`collectFilesRecursive()`** - Helper that traverses directories
- **Container processing** - Compiles MDX via `BUNDLER_CONTAINER`

### Repository Structure

Blog posts are stored in GitHub at: `content/blog/{post-name}/index.mdx`

## Migration Strategy

### Current Architecture (HTTP-based)

```
Orchestrator (webhook)
  → env.BUNDLER.fetch(new Request(...))
    → Bundler.fetch() handler
      → Parse JSON, validate
      → Process posts
      → Return Response
```

### New Architecture (Pure RPC)

```
Orchestrator (webhook)
  → env.BUNDLER.bundlePosts(postChanges)  // Direct RPC call
    → Bundler WorkerEntrypoint.bundlePosts()
      → Process posts (no Request/Response)
      → Return counts

Resolver (cache miss)
  → env.BUNDLER.bundleAllPosts(owner, repo)  // Direct RPC call
    → Bundler WorkerEntrypoint.bundleAllPosts()
      → Get all post names from GitHub
      → Process all posts
      → Return counts
```

### Key Benefits of This Approach

- **Full type safety**: Both methods use direct TypeScript parameters (no Request/Response)
- **Simpler code**: No JSON serialization/deserialization needed
- **Cleaner architecture**: Pure RPC, no HTTP layer complexity
- **Zero overhead**: Same performance, runs on same thread
- **Code reuse**: Both methods use shared `processPosts()` logic

### Breaking Changes

⚠️ **Orchestrator must be updated** to use RPC instead of fetch:

- Before: `env.BUNDLER.fetch(new Request(url, { body: JSON.stringify(posts) }))`
- After: `env.BUNDLER.bundlePosts(posts)`

## Implementation Plan

### Phase 1: Extract Shared Bundling Logic

**File**: `/workers/bundler/src/bundler.ts` (new file)

**Create shared function**: `processPosts()`

```typescript
/**
 * Shared bundling logic used by both bundlePosts() and bundleAllPosts()
 * Processes an array of PostChange objects through the bundling pipeline
 */
export async function processPosts(
  env: Env,
  posts: PostChange[],
): Promise<{
  totalAddedOrModified: number;
  totalRemoved: number;
  metadata: PostMetadata[];
}>;
```

**Implementation**:

- Extract lines 43-103 from current `fetch()` handler
- Return both counts AND metadata array for cache update
- Add metadata extraction during processing
- This is the EXACT same logic, just extracted to a reusable function

### Phase 2: Create GitHub Utility Function

**File**: `/workers/bundler/src/github.ts`

**Add new function**: `getAllBlogPostNames()`

```typescript
/**
 * Fetches all blog post folder names from the GitHub repository
 * Uses repos.getContent API to list folders in content/blog directory
 * @param client - Octokit instance
 * @param owner - Repository owner
 * @param repo - Repository name
 * @returns Array of post folder names (e.g., ["hello-world", "my-post"])
 */
export async function getAllBlogPostNames(
  client: Octokit,
  owner: string,
  repo: string,
): Promise<string[]> {
  const { data } = await client.repos.getContent({
    owner,
    repo,
    path: "content/blog",
  });

  if (!Array.isArray(data)) {
    return [];
  }

  // Every item is a post folder - just extract the names
  return data.map((item) => item.name);
}
```

**Implementation approach**:

1. Use `client.repos.getContent()` with `path: "content/blog"`
2. Returns array of all immediate children in the directory
3. Map each item to its `name` property (the post folder name)
4. Return array of post names

**Why repos.getContent API?**

- Perfect fit: Returns exactly what we need (folder names in `content/blog/`)
- Simple and direct: One API call, no filtering needed
- Repository structure guarantee: `content/blog/` contains only post folders
- Limit: 1,000 folders (sufficient for a blog - unlikely to exceed)
- The existing `fetch()` function already expects this format and handles recursive file fetching via `collectPostFiles()`

### Phase 3: Migrate to WorkerEntrypoint with RPC Methods

**File**: `/workers/bundler/src/index.ts`

**Replace fetch handler with WorkerEntrypoint class**:

```typescript
import { WorkerEntrypoint } from "cloudflare:workers";
import { Container, getRandom } from "@cloudflare/containers";
import { getOctokit } from "./octokit";
import {
  type PostChange,
  type PostMetadata,
  validatePostChanges,
} from "@repo/schemas";
import { getAllBlogPostNames } from "./github";
import { processPosts } from "./bundler";

export interface Env {
  GITHUB_TOKEN: string;
  POSTS_CACHE: KVNamespace;
  BUNDLER_CONTAINER: DurableObjectNamespace<BundlerContainer>;
  GITHUB_OWNER?: string; // Optional env vars for convenience
  GITHUB_REPO?: string;
}

export class BundlerContainer extends Container<Env> {
  defaultPort = 8080;
  sleepAfter = "1m";
}

// Pure RPC worker using WorkerEntrypoint
export default class extends WorkerEntrypoint<Env> {
  /**
   * RPC Method 1: Bundle specific posts (webhook-driven)
   * Called by orchestrator worker via env.BUNDLER.bundlePosts()
   * @param posts - Array of post changes to process
   * @returns Processing summary
   */
  async bundlePosts(posts: PostChange[]): Promise<{
    totalAddedOrModified: number;
    totalRemoved: number;
  }> {
    if (!posts.length) {
      return { totalAddedOrModified: 0, totalRemoved: 0 };
    }

    const { totalAddedOrModified, totalRemoved, metadata } = await processPosts(
      this.env,
      posts,
    );

    // Update metadata cache
    await this.env.POSTS_CACHE.put("blog:posts:all", JSON.stringify(metadata));

    return { totalAddedOrModified, totalRemoved };
  }

  /**
   * RPC Method 2: Bundle all posts (manual refresh / cache miss)
   * Called by resolver worker on cache miss or manual trigger
   * @param owner - GitHub repository owner
   * @param repo - GitHub repository name
   * @returns Processing summary
   */
  async bundleAllPosts(
    owner: string,
    repo: string,
  ): Promise<{
    totalAddedOrModified: number;
    totalRemoved: number;
  }> {
    const client = getOctokit(this.env.GITHUB_TOKEN);

    if (!client) {
      throw new Error("Failed to initialize GitHub client");
    }

    // Get all blog post folder names
    const postNames = await getAllBlogPostNames(client, owner, repo);

    // Convert to PostChange[] format
    const posts: PostChange[] = postNames.map((postName) => ({
      postName,
      ownerName: owner,
      repoName: repo,
    }));

    // Process all posts using shared logic
    const { totalAddedOrModified, totalRemoved, metadata } = await processPosts(
      this.env,
      posts,
    );

    // Update metadata cache
    await this.env.POSTS_CACHE.put("blog:posts:all", JSON.stringify(metadata));

    return { totalAddedOrModified, totalRemoved };
  }
}
```

**Key architecture**:

- Pure RPC worker - no HTTP fetch handler
- `bundlePosts()` - Direct RPC call with PostChange[] array
- `bundleAllPosts()` - Direct RPC call with owner/repo parameters
- Both methods call shared `processPosts()` logic
- Both methods update `blog:posts:all` metadata cache

### Phase 4: Extract Metadata Helper Function

**File**: `/workers/bundler/src/metadata.ts` (new file)

**Create function**: `extractPostMetadata()`

```typescript
/**
 * Extracts PostMetadata from MDX source frontmatter
 * @param postName - Post folder name
 * @param source - MDX source content
 * @returns PostMetadata object
 */
export function extractPostMetadata(
  postName: string,
  source: string,
): PostMetadata;
```

**Implementation**:

1. Parse frontmatter from MDX source (if exists)
2. Extract: `title`, `date`, `description`, `tags`
3. Return PostMetadata object (from `@repo/schemas`)
4. Handle missing/invalid frontmatter gracefully

**Frontmatter parsing**:

- Option A: Use simple regex to extract YAML frontmatter block
- Option B: Use a lightweight frontmatter parser (add dependency)
- **Recommendation**: Start with Option A (regex) for simplicity

### Phase 5: Update Orchestrator to Use RPC

**File**: `/workers/orchestrator/src/index.ts`

**Before (fetch-based)**:

```typescript
ctx.waitUntil(
  env.BUNDLER.fetch(
    new Request(request.url, {
      body: JSON.stringify(postChanges),
      method: "POST",
    }),
  ),
);
```

**After (RPC-based)**:

```typescript
ctx.waitUntil(env.BUNDLER.bundlePosts(postChanges));
```

**Changes**:

- Remove `new Request()` wrapping
- Remove `JSON.stringify()` serialization
- Direct RPC method call with TypeScript types
- Much simpler and type-safe!

### Phase 6: Update Resolver to Call RPC Method

**File**: `/workers/resolver/src/index.ts`

**Update `getPosts()` to call bundleAllPosts**:

```typescript
async getPosts(): Promise<PostMetadata[]> {
  const cached = await this.env.POSTS_CACHE.get('blog:posts:all', 'json');

  if (cached) {
    return validatePostsList(cached);
  }

  // Cache miss - call bundler to refresh all posts via RPC
  const owner = this.env.GITHUB_OWNER || "default-owner";  // From env vars
  const repo = this.env.GITHUB_REPO || "default-repo";

  await this.env.BUNDLER.bundleAllPosts(owner, repo);

  // Re-fetch from cache
  const updated = await this.env.POSTS_CACHE.get('blog:posts:all', 'json');
  return validatePostsList(updated ?? []);
}
```

**Add to Resolver Env interface**:

```typescript
export interface Env {
  POSTS_CACHE: KVNamespace;
  BUNDLER: Fetcher; // Service binding supports RPC
  GITHUB_OWNER?: string; // For bundleAllPosts
  GITHUB_REPO?: string;
}
```

## File Changes Summary

### New Files

1. `/workers/bundler/src/bundler.ts` - **NEW**: Shared bundling logic (`processPosts()`)
2. `/workers/bundler/src/metadata.ts` - Metadata extraction utilities
3. `/workers/bundler/IMPLEMENTATION_PLAN.md` - This document

### Modified Files (Bundler Worker)

1. `/workers/bundler/src/index.ts` - **BREAKING CHANGE**

   - Replace `export default { async fetch() }` with `export default class extends WorkerEntrypoint<Env>`
   - Replace `fetch()` handler with `bundlePosts()` RPC method
   - Add `bundleAllPosts()` RPC method (new functionality)
   - Both methods call shared `processPosts()` function

2. `/workers/bundler/src/github.ts`
   - Add `getAllBlogPostNames()` function

### Modified Files (Orchestrator Worker)

3. `/workers/orchestrator/src/index.ts` - **REQUIRED UPDATE**
   - Change from `env.BUNDLER.fetch(new Request(...))` to `env.BUNDLER.bundlePosts(postChanges)`
   - Remove Request wrapping and JSON serialization
   - Direct RPC method call

### Modified Files (Resolver Worker)

4. `/workers/resolver/src/index.ts` - **SIMPLE UPDATE**
   - Update `getPosts()` to call `env.BUNDLER.bundleAllPosts(owner, repo)` on cache miss
   - Add `GITHUB_OWNER` and `GITHUB_REPO` to Env interface

### Wrangler Configuration Updates

5. `/workers/resolver/wrangler.jsonc`
   - Add environment variables for `GITHUB_OWNER` and `GITHUB_REPO` (optional)

### Dependencies

- **No new packages required**
- Add `import { WorkerEntrypoint } from 'cloudflare:workers'` to bundler
- Leverage existing Octokit, Zod, @repo/schemas, etc.

## Testing Strategy

### Unit Tests

1. Test `getAllBlogPosts()` with mocked Octokit responses
2. Test `extractPostMetadata()` with various frontmatter formats
3. Test `bundleAllPosts()` with mocked GitHub and container responses

### Integration Tests

1. Test full flow with a test repository
2. Verify KV cache updates correctly
3. Test error handling (missing posts, network failures)

### Manual Testing

1. Deploy to staging environment
2. Trigger `bundleAllPosts()` via resolver worker
3. Verify `blog:posts:all` cache contents
4. Verify individual post caches at `mdx:post:{postName}`

## Edge Cases & Error Handling

1. **Empty repository** - Return empty metadata array
2. **Invalid post folders** - Skip posts without `index.mdx`
3. **Compilation failures** - Log error, continue processing other posts
4. **API rate limits** - Git Tree API uses 1 request vs many with Contents API
5. **Large repositories** - Git Tree API handles this efficiently
6. **Concurrent updates** - Consider using KV TTL for cache invalidation
7. **Deleted posts** - Don't include in metadata array, delete from KV

## Performance Considerations

- **API efficiency**: Single Git Tree API call vs O(n) Contents API calls
- **Parallel processing**: Consider processing posts concurrently (Promise.all)
- **Container scaling**: Uses existing `getRandom(env.BUNDLER_CONTAINER, 3)` logic
- **KV operations**: Batch writes where possible

## Future Enhancements

1. **Incremental updates** - Compare with existing cache to skip unchanged posts
2. **Background refresh** - Periodic cache refresh via Cron Triggers
3. **Webhook integration** - Update metadata cache on every webhook call
4. **Pagination** - Handle large metadata arrays efficiently
5. **Sorting** - Sort posts by date in metadata array

## Questions for Review

1. Should `owner` and `repo` be environment variables or parameters?

   - **Recommendation**: Parameters for flexibility, env vars as fallback

2. Should we extract metadata during webhook processing too?

   - **Recommendation**: Yes, update `blog:posts:all` on every webhook

3. How to handle frontmatter parsing - regex or library?

   - **Recommendation**: Start with regex, add library if needed

4. Should post processing be parallel or sequential?

   - **Recommendation**: Sequential for now (existing pattern), parallel later

5. What should happen if metadata extraction fails?
   - **Recommendation**: Use postName only, log warning, continue processing

## Implementation Order

### Step 1: Prepare Helper Functions (No Breaking Changes)

- [ ] Create `extractPostMetadata()` in `/workers/bundler/src/metadata.ts`
- [ ] Create `getAllBlogPostNames()` in `/workers/bundler/src/github.ts`
- [ ] Extract shared logic to `processPosts()` in `/workers/bundler/src/bundler.ts`

### Step 2: Migrate Bundler to Pure RPC (BREAKING CHANGE!)

- [ ] Update `/workers/bundler/src/index.ts` to use WorkerEntrypoint
- [ ] Replace `fetch()` handler with `bundlePosts()` RPC method
- [ ] Add `bundleAllPosts()` RPC method (new)
- [ ] Test bundler worker locally with wrangler dev

### Step 3: Update Orchestrator (REQUIRED - Must Deploy with Bundler)

- [ ] Update `/workers/orchestrator/src/index.ts` to call RPC method
- [ ] Change from `env.BUNDLER.fetch(...)` to `env.BUNDLER.bundlePosts(postChanges)`
- [ ] Test webhook flow end-to-end
- [ ] Deploy orchestrator with bundler (coordinated deployment)

### Step 4: Update Resolver (Independent)

- [ ] Update `/workers/resolver/src/index.ts` to call `bundleAllPosts()`
- [ ] Add `GITHUB_OWNER` and `GITHUB_REPO` env vars to wrangler.jsonc
- [ ] Test cache miss scenario
- [ ] Deploy resolver

### Step 5: Verification

- [ ] Trigger webhook to test new `bundlePosts()` RPC flow
- [ ] Clear cache and test `bundleAllPosts()` RPC flow via resolver
- [ ] Verify metadata cache updates correctly
- [ ] Monitor logs for errors

**⚠️ Important**: Bundler + Orchestrator must be deployed together due to breaking changes!

## Dependencies on Other Work

- Resolver worker already expects `blog:posts:all` cache (implemented)
- PostMetadata schema already defined in `@repo/schemas` (implemented)
- Existing bundling pipeline is stable and working

## Rollout Plan

1. Implement and test in development environment
2. **Deploy bundler + orchestrator together** (breaking change - coordinated deployment)
3. Verify webhook flow works with new RPC calls
4. Deploy resolver worker to enable `bundleAllPosts()` functionality
5. Manual trigger of `bundleAllPosts()` to populate initial cache
6. Monitor logs and KV cache behavior
7. Optional: Add scheduled refresh via Cron Triggers

**Critical**: Bundler and orchestrator must be deployed simultaneously to avoid breaking the webhook flow.

---

**Document Version**: 3.0
**Created**: 2025-10-12
**Updated**: 2025-10-12 (Pure RPC Migration)
**Author**: Claude Code
**Status**: Ready for Implementation

## Summary

This plan migrates the bundler worker to **pure RPC architecture** with two major changes:

### 1. Pure RPC Migration (Breaking Change)

- Convert bundler to use `WorkerEntrypoint` class
- **Replace `fetch()` handler with `bundlePosts()` RPC method** ⚠️
- **Orchestrator MUST be updated** to call RPC instead of fetch
- Benefits: Full type safety, simpler code, cleaner architecture

### 2. Bundle All Posts Feature (New Functionality)

- Add `bundleAllPosts()` RPC method for full cache refresh
- Leverages **90% existing code** via shared `processPosts()` function
- Uses `repos.getContent()` to get all post folder names (simple, one API call)
- Called by resolver worker on cache miss

### Architecture Benefits

- **Pure RPC**: No HTTP layer complexity, direct TypeScript method calls
- **Code reuse**: Both `bundlePosts()` and `bundleAllPosts()` use same `processPosts()` logic
- **Full type safety**: No Request/Response serialization/deserialization
- **Metadata caching**: Both methods update `blog:posts:all` cache automatically
- **Simpler code**: Remove JSON.stringify/parse, Request wrapping, error handling

### Migration Strategy

1. Create helper functions first (no breaking changes)
2. Migrate bundler to pure RPC (**breaking change**)
3. **Deploy bundler + orchestrator together** (coordinated deployment required)
4. Update resolver to use RPC method (independent deployment)
5. Deploy order: bundler + orchestrator → resolver

⚠️ **Breaking Change**: This is NOT backward compatible. Bundler and orchestrator must be deployed simultaneously to avoid breaking the webhook flow.
