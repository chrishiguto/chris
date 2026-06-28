# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a personal website and blog built with a Vite + React frontend and Cloudflare Workers backend. The system processes GitHub webhook events to compile and cache MDX blog posts.

## Repository Structure

This is a pnpm monorepo using Turborepo with three main workspace types:

- **apps/web** - Vite + React frontend application with safe-mdx rendering
- **packages/** - Shared packages (ui, eslint-config, typescript-config)
- **workers/** - Three Cloudflare Workers that form a blog compilation pipeline:
  - **conductor** - Entry point worker that validates GitHub webhooks and routes blog post changes
  - **refresher** - Worker entrypoint that fetches post content from GitHub API
  - **compiler** - Container-enabled worker that compiles MDX; uses Cloudflare Containers with Bun runtime (container source in `container-src/`)
- **content/blog/** - MDX blog posts stored in `content/blog/{post-name}/index.mdx` format

## Common Commands

### Development
```bash
pnpm dev              # Start all apps/workers in dev mode (turbo)
pnpm build            # Build all packages/apps (turbo)
pnpm lint             # Lint all packages/apps (turbo)
pnpm format           # Format code with Prettier
```

### Web App (apps/web)
```bash
cd apps/web
pnpm dev              # Start Vite dev server (--clearScreen false)
pnpm build            # TypeScript compile + Vite build
pnpm preview          # Preview production build
pnpm lint             # Lint TypeScript files
```

### Cloudflare Workers (workers/*)
All three workers (conductor, refresher, compiler) use the same commands:
```bash
cd workers/{worker-name}
pnpm dev              # Run wrangler dev server
pnpm deploy           # Deploy to Cloudflare
pnpm test             # Run vitest tests
pnpm cf-typegen       # Generate Cloudflare types
```

### Compiler Container (workers/compiler/container-src)
The container runs in Cloudflare's container runtime with Bun:
```bash
cd workers/compiler/container-src
bun run dev           # Start Bun dev server with hot reload
```

## Architecture

### Blog Compilation Pipeline

1. **GitHub Push Event** → **Conductor Worker**
   - Validates webhook signature using `@octokit/webhooks`
   - Filters for main branch pushes
   - Detects changes to `content/blog/*/index.mdx` files
   - Forwards post changes to Compiler worker via service binding

2. **Compiler Worker**
   - Receives post change notifications
   - Fetches MDX content from GitHub API using `@octokit/rest`
   - Forwards compilation work to Container instances
   - Uses Durable Objects for container orchestration (`@cloudflare/containers`)

3. **Refresher Worker** (WorkerEntrypoint)
   - Legacy/alternative entrypoint for post processing
   - Parses MDX with `safe-mdx/parse`
   - Stores compiled AST and source code in `POSTS_CACHE` KV namespace with keys:
     - `mdx:post:ast:{postName}` - Parsed MDX AST
     - `mdx:post:code:{postName}` - Raw MDX source

4. **Container Runtime** (Bun + Hono)
   - Runs in Cloudflare Container (defined in `workers/compiler/container-src/`)
   - Handles actual MDX compilation work
   - Uses Hono framework for routing

### Frontend Architecture

The web app (`apps/web`) uses:
- React with TypeScript
- Vite for bundling
- `SafeMdxRenderer` from `safe-mdx` to render MDX AST
- Shared UI components from `@repo/ui` package
- Currently includes hardcoded AST example in main.tsx

### Worker Bindings

- **conductor** → binds to **compiler** worker via service binding (`COMPILER`)
- **refresher** → uses `GITHUB_TOKEN` secret and `POSTS_CACHE` KV namespace
- **compiler** → uses `GITHUB_TOKEN`, `POSTS_CACHE` KV, and `COMPILER_CONTAINER` Durable Object

## Testing

Workers use Vitest with `@cloudflare/vitest-pool-workers` for testing in the Workers runtime environment. Test files are in `workers/{name}/test/` directories.

## Deployment

- Workers deploy individually via `wrangler deploy` from their respective directories
- Frontend deploys via standard Vite build process
- All workers use `wrangler.jsonc` for configuration with compatibility flags: `nodejs_compat`, `nodejs_compat_do_not_populate_process_env`