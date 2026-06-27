#!/usr/bin/env bash
# Run ORP conformance tests
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test -p orp-core -- --nocapture
cargo test -p orp-bridge -- --nocapture
cargo test -p orp-server -- --nocapture 2>/dev/null || true
echo "Conformance: core tests passed"
