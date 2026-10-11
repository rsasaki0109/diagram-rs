#!/usr/bin/env bash
# Build and validate local archives. This never publishes or pushes.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --locked -p diagram-cli
scripts/build-wasm.sh
mkdir -p dist
release_npm_cache=$(mktemp -d /tmp/diagram-rs-npm.XXXXXX)
trap 'rm -rf "$release_npm_cache"' EXIT
npm pack ./crates/diagram-wasm/pkg --cache "$release_npm_cache" --offline --ignore-scripts --pack-destination dist >/dev/null
python3 scripts/package-release.py
