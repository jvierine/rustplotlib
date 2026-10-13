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
mkdir -p web/gallery
cp gallery/fields.{pdf,png,jpg,svg} gallery/single_column.pdf web/gallery/
