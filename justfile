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

# deploy to cloudflare workers
deploy:
    npx wrangler deploy

# gzipped wasm sizes — the server number is what the workers plan limit cares about
size:
    @echo "server worker: $(gzip -9 -c workers/site/build/index_bg.wasm | wc -c) bytes gzipped"
    @echo "client islands: $(gzip -9 -c target/site/pkg/{{output_name}}.wasm | wc -c) bytes gzipped"

# one-time toolchain setup (rust + node assumed)
setup:
    rustup target add wasm32-unknown-unknown
    cargo install --locked cargo-leptos worker-build
