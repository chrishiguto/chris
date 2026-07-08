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

# the write-path worker (the /publish reconcile op)
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
    sha="manual-$(git rev-parse --short=12 HEAD)-$(date +%s)"
    cargo run -q -p xtask -- plan --sha "$sha" --out "$out"
    npx wrangler kv bulk put --binding BLOG {{remote}} "$out/writes.json"
    # the pointer flips only after every snapshot key landed
    npx wrangler kv key put --binding BLOG {{remote}} current --path "$out/pointer.json"
    # Workers Cache is purgeable only from inside the site worker; a manual
    # publish bypasses the coordinator's diff, so it purges the whole site tag.
    if [ -n "${SITE_ORIGIN:-}" ] && { [ -n "${PURGE_SHARED_SECRET:-}" ] || [ -f .secrets/purge_shared_secret ]; }; then
        just purge && echo "site cache purged" \
            || echo "warning: purge failed — cached pages fall back to the 7-day TTL"
    else
        echo "purge skipped (SITE_ORIGIN or the purge secret missing) — TTL or the CI purge converges"
    fi

# break-glass full cache purge (the `site` tag); publishes and deploys purge
# their own scopes — this is for everything else. The secret comes from
# $PURGE_SHARED_SECRET (CI) or .secrets/purge_shared_secret (local).
purge:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${SITE_ORIGIN:?set SITE_ORIGIN to the deployed site origin}"
    secret="${PURGE_SHARED_SECRET:-$(cat .secrets/purge_shared_secret)}"
    curl -sf -X POST "${SITE_ORIGIN%/}/__purge" \
        -H "Authorization: Bearer $secret" \
        -H "Content-Type: application/json" \
        --data '{"tags":["site"]}' > /dev/null

# one-time toolchain setup (rust + node assumed)
setup:
    rustup target add wasm32-unknown-unknown
    cargo install --locked cargo-leptos worker-build
