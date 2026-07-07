# wrangler.toml's [build] also calls `just build` — dev, deploy, and ci share one recipe.

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

# gzipped wasm sizes — worker scripts fail past the 10 MB Workers limit, warn
# past 5 MB; client islands are assets with no script limit.
size:
    #!/usr/bin/env bash
    set -euo pipefail
    budget() {
        local label=$1 file=$2 size
        size=$(gzip -9 -c "$file" | wc -c)
        echo "$label: $size bytes gzipped"
        if [ "$size" -gt 10485760 ]; then
            echo "${GITHUB_ACTIONS:+::error::}$label exceeds the 10 MB gzipped Workers limit ($size bytes)"
            exit 1
        elif [ "$size" -gt 5242880 ]; then
            echo "${GITHUB_ACTIONS:+::warning::}$label is over 5 MB gzipped ($size bytes), approaching the 10 MB limit"
        fi
    }
    budget "server worker" workers/site/build/index_bg.wasm
    echo "client islands: $(gzip -9 -c target/site/pkg/{{output_name}}.wasm | wc -c) bytes gzipped"
    budget "pipeline worker" workers/pipeline/build/index_bg.wasm

# format everything; leptosfmt covers content/ because rustfmt can't see the
# build.rs-included per-post components
fmt:
    leptosfmt app workers content
    cargo fmt --all

# fmt-check + clippy + content validation (native target — compiles only
# because ssr deps are feature-gated)
check:
    leptosfmt --check app workers content
    cargo fmt --all --check
    cargo clippy --workspace -- -D warnings
    cargo run -q -p xtask -- check

# native test suite; feature unification keeps the gated suites in this run
test:
    cargo test --workspace

# break-glass publish straight to KV, bypassing the coordinator — for when the
# pipeline itself is broken; the next reconcile supersedes it.
# `remote='--local'` targets the `wrangler dev` simulator instead.
remote := '--remote'

publish:
    #!/usr/bin/env bash
    set -euo pipefail
    out=target/publish
    mkdir -p "$out"
    # missing keys print "Value not found" with exit 0 — the only non-JSON output xtask accepts
    npx wrangler kv key get --binding BLOG {{remote}} current --text > "$out/current.json"
    # xtask names the key so its grammar stays out of bash
    prev_index_key=$(cargo run -q -p xtask -- pointer "$out/current.json")
    npx wrangler kv key get --binding BLOG {{remote}} "$prev_index_key" --text > "$out/index.json"
    sha="manual-$(git rev-parse --short=12 HEAD)-$(date +%s)"
    cargo run -q -p xtask -- plan --sha "$sha" --index "$out/index.json" --out "$out" ${SITE_ORIGIN:+--origin "$SITE_ORIGIN"}
    npx wrangler kv bulk put --binding BLOG {{remote}} "$out/writes.json"
    # the pointer flips only after every snapshot key landed
    npx wrangler kv key put --binding BLOG {{remote}} current --path "$out/pointer.json"
    if [ -n "${CLOUDFLARE_ZONE_ID:-}" ] && [ -n "${SITE_ORIGIN:-}" ]; then
        # one request per API-capped chunk; a failed purge just leaves the 7-day TTL backstop
        purged=1
        for chunk in "$out"/purge-*.json; do
            curl -sf -X POST "https://api.cloudflare.com/client/v4/zones/$CLOUDFLARE_ZONE_ID/purge_cache" \
                -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
                -H "Content-Type: application/json" \
                -d "{\"files\": $(cat "$chunk")}" > /dev/null || purged=0
        done
        if [ "$purged" = 1 ]; then
            echo "purged the plan's urls"
        else
            echo "warning: purge failed — cached pages fall back to the 7-day TTL"
        fi
    else
        echo "purge skipped (CLOUDFLARE_ZONE_ID/SITE_ORIGIN unset — inert on workers.dev)"
    fi

# one-time toolchain setup (rust + node assumed)
setup:
    rustup target add wasm32-unknown-unknown
    cargo install --locked cargo-leptos worker-build
