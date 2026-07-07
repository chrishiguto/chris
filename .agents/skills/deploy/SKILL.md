---
name: deploy
description: Build, verify wasm size budget, and deploy the site worker to Cloudflare. Use for "deploy the site" / "ship it".
disable-model-invocation: true
---

Deploy the site worker to Cloudflare. Steps, in order — stop at the first failure and report it:

1. Warn if the working tree is dirty (`git status --porcelain`) — deploying uncommitted changes is allowed but should be deliberate.
2. `just build` — builds the client bundle (cargo-leptos) and the SSR worker (worker-build).
3. `just size` — report both numbers. The server artifact (`workers/site/build/index_bg.wasm` gzipped) must be under the Workers 10 MB limit; abort if it exceeds it, and warn if it's within 20% of the limit.
4. `just deploy` — deploys via wrangler.
5. Confirm the deploy by fetching the production URL (https://chris-site.sirhc.workers.dev) and checking for a 200 with HTML content.

Report the deployed size numbers and the URL when done.
