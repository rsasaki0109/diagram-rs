#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
command -v wasm-bindgen >/dev/null || { echo 'Install wasm-bindgen-cli 0.2.105 (see README)' >&2; exit 1; }
[[ "$(wasm-bindgen --version)" == 'wasm-bindgen 0.2.105' ]] || { echo 'wasm-bindgen-cli must be 0.2.105' >&2; exit 1; }
cargo build --locked --release -p diagram-wasm --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir crates/diagram-wasm/pkg --out-name diagram_rs target/wasm32-unknown-unknown/release/diagram_wasm.wasm
cp crates/diagram-wasm/js/* crates/diagram-wasm/pkg/
cp LICENSE crates/diagram-wasm/pkg/LICENSE
cp THIRD_PARTY_LICENSES.txt crates/diagram-wasm/pkg/THIRD_PARTY_LICENSES.txt
cp assets/fonts/NotoSansJP-OFL.txt crates/diagram-wasm/pkg/NOTO-FONT-LICENSE.txt
cp assets/fonts/LICENSE.txt crates/diagram-wasm/pkg/FONT-LICENSE.txt
cp crates/diagram-wasm/README.md crates/diagram-wasm/pkg/README.md
