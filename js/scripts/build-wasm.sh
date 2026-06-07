#!/usr/bin/env bash
# Build the alt-markdown WASM core and generate browser (web-target) bindings.
# Requires wasm-bindgen-cli matching the wasm-bindgen crate version:
#   cargo install wasm-bindgen-cli --version <version from Cargo.lock>
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

cargo build -p altmd-wasm --release --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/release/altmd_wasm.wasm \
  --out-dir js/wasm/web --target web

echo "Generated js/wasm/web (web-target bindings)."
