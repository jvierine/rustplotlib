#!/usr/bin/env bash
set -Eeuo pipefail
cd "$(dirname "$0")/.."
command -v wasm-bindgen >/dev/null || { echo 'Install wasm-bindgen-cli 0.2.129: cargo install wasm-bindgen-cli --version 0.2.129 --locked'; exit 1; }
cargo build -p rustplotlib-web --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir web/pkg target/wasm32-unknown-unknown/release/rustplotlib_web.wasm
mkdir -p web/dist
npx esbuild web/src/app.ts --bundle --format=esm --outfile=web/dist/app.js
cp web/pkg/rustplotlib_web_bg.wasm web/dist/
cargo run --release --example web_data
cargo run --release --example surfaces
cargo run --release --example gallery
mkdir -p web/gallery
cp -R gallery/* web/gallery/
cargo run --release --example atmosphere
npx esbuild web/src/heimdall.ts --bundle --format=esm --outfile=web/dist/heimdall.js
npx esbuild web/src/atmosphere.ts --bundle --format=esm --outfile=web/dist/atmosphere.js

cargo run --release --example heimdall_labels
