# the build pipeline lives in `just build`; wrangler.toml calls it too, so dev,
# deploy, and ci all go through the same recipe.

wasm_rustflags := '--cfg getrandom_backend="wasm_js"'

# must match output-name in [[workspace.metadata.leptos]] (Cargo.toml)
output_name := 'chris'

default:
    @just --list

# serve the site locally (ssr + hydration) at http://localhost:8787
dev:
    npx wrangler dev

# frontend wasm + tailwind (cargo-leptos), then the ssr worker (worker-build)
build:
    CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS='{{wasm_rustflags}}' cargo leptos build --release
    cd workers/site && LEPTOS_OUTPUT_NAME={{output_name}} CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS='{{wasm_rustflags}}' worker-build --release --features ssr

# the write-path worker (webhook + publish op)
build-pipeline:
    cd workers/pipeline && CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS='{{wasm_rustflags}}' worker-build --release --features worker

# deploy to cloudflare workers
deploy:
    npx wrangler deploy

deploy-pipeline:
    npx wrangler deploy --config workers/pipeline/wrangler.toml

# gzipped wasm sizes — the server number is what the workers plan limit cares about
size:
    @echo "server worker: $(gzip -9 -c workers/site/build/index_bg.wasm | wc -c) bytes gzipped"
    @echo "client islands: $(gzip -9 -c target/site/pkg/{{output_name}}.wasm | wc -c) bytes gzipped"
    @echo "pipeline worker: $(gzip -9 -c workers/pipeline/build/index_bg.wasm | wc -c) bytes gzipped"

# format everything (leptosfmt handles view! macros, rustfmt the rest;
# content/ holds co-located per-post components — rustfmt can't see them
# through the build.rs include!, so leptosfmt covers them here)
fmt:
    leptosfmt app workers content
    cargo fmt --all

# fmt-check + clippy (native target; ssr deps are feature-gated so this compiles)
# + content-tree validation against the compiled component vocabulary
check:
    leptosfmt --check app workers content
    cargo fmt --all --check
    cargo clippy --workspace -- -D warnings
    cargo run -q -p xtask -- check

# break-glass publish straight to KV through wrangler: xtask plans the whole
# tree as one snapshot, wrangler moves the bytes and flips `current` last.
# Deliberately bypasses the coordinator — the escape hatch for when the
# pipeline worker itself is the problem; the next reconcile supersedes it.
# `remote='--local'` targets the `wrangler dev` simulator instead.
remote := '--remote'

publish:
    #!/usr/bin/env bash
    set -euo pipefail
    out=target/publish
    mkdir -p "$out"
    # Missing keys print "Value not found" and exit 0 — the only non-JSON
    # output xtask accepts; anything else fails before reaching `plan`.
    npx wrangler kv key get --binding BLOG {{remote}} current --text > "$out/current.json"
    # xtask names the key holding the previous index; bash just reads it.
    prev_index_key=$(cargo run -q -p xtask -- pointer "$out/current.json")
    npx wrangler kv key get --binding BLOG {{remote}} "$prev_index_key" --text > "$out/index.json"
    sha="manual-$(git rev-parse --short=12 HEAD)-$(date +%s)"
    cargo run -q -p xtask -- plan --sha "$sha" --index "$out/index.json" --out "$out" ${SITE_ORIGIN:+--origin "$SITE_ORIGIN"}
    npx wrangler kv bulk put --binding BLOG {{remote}} "$out/writes.json"
    # The pointer flips only after every snapshot key landed.
    npx wrangler kv key put --binding BLOG {{remote}} current --path "$out/pointer.json"
    if [ -n "${CLOUDFLARE_ZONE_ID:-}" ] && [ -n "${SITE_ORIGIN:-}" ]; then
        # A failed purge only means the 7-day TTL backstop.
        curl -sf -X POST "https://api.cloudflare.com/client/v4/zones/$CLOUDFLARE_ZONE_ID/purge_cache" \
            -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
            -H "Content-Type: application/json" \
            -d "{\"files\": $(cat "$out/purge.json")}" > /dev/null \
            && echo "purged the plan's urls" \
            || echo "warning: purge failed — cached pages fall back to the 7-day TTL"
    else
        echo "purge skipped (CLOUDFLARE_ZONE_ID/SITE_ORIGIN unset — inert on workers.dev)"
    fi

# one-time toolchain setup (rust + node assumed)
setup:
    rustup target add wasm32-unknown-unknown
    cargo install --locked cargo-leptos worker-build
