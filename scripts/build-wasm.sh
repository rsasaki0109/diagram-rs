#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
wasm_package_dir=crates/diagram-wasm/pkg
wasm_feature_args=()
if [[ "${1:-}" == --latin-only && $# == 1 ]]; then
  wasm_package_dir=crates/diagram-wasm/pkg-latin
  wasm_feature_args=(--no-default-features)
elif (( $# > 0 )); then
  echo 'Usage: scripts/build-wasm.sh [--latin-only]' >&2
  exit 2
fi
command -v wasm-bindgen >/dev/null || { echo 'Install wasm-bindgen-cli 0.2.105 (see README)' >&2; exit 1; }
[[ "$(wasm-bindgen --version)" == 'wasm-bindgen 0.2.105' ]] || { echo 'wasm-bindgen-cli must be 0.2.105' >&2; exit 1; }
cargo build --locked --release -p diagram-wasm --target wasm32-unknown-unknown "${wasm_feature_args[@]}"
wasm-bindgen --target web --out-dir "$wasm_package_dir" --out-name diagram_rs target/wasm32-unknown-unknown/release/diagram_wasm.wasm
cp crates/diagram-wasm/js/* "$wasm_package_dir/"
cp LICENSE "$wasm_package_dir/LICENSE"
cp THIRD_PARTY_LICENSES.txt "$wasm_package_dir/THIRD_PARTY_LICENSES.txt"
cp assets/fonts/NotoSansJP-OFL.txt "$wasm_package_dir/NOTO-FONT-LICENSE.txt"
cp assets/fonts/LICENSE.txt "$wasm_package_dir/FONT-LICENSE.txt"
cp crates/diagram-wasm/README.md "$wasm_package_dir/README.md"
if [[ "$wasm_package_dir" == */pkg-latin ]]; then
  printf '\nThis package uses the Latin-only build. Japanese requires renderWithFont.\n' >> "$wasm_package_dir/README.md"
fi
