#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown 2>/dev/null || true
cargo build -p orp-wasm --target wasm32-unknown-unknown --release
echo "WASM built: target/wasm32-unknown-unknown/release/orp_wasm.wasm"
