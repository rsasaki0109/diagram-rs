#!/usr/bin/env bash
# Reproducible setup; run from any directory. Does not edit source or baselines.
set -euo pipefail
cd /workspace/diagram-rs
export CARGO_HOME=/workspace/.diagram-tools/cargo
export RUSTUP_HOME=/workspace/.diagram-tools/rustup
export PATH="$CARGO_HOME/bin:/workspace/.diagram-tools/wasm-bindgen/bin:$PATH"
export CARGO_BUILD_JOBS=4
if [[ ! -x "$CARGO_HOME/bin/rustup" ]]; then
  installer=$(mktemp /tmp/diagram-rustup.XXXXXX)
  trap 'rm -f "$installer"' EXIT
  curl --fail --location --proto '=https' --tlsv1.2 https://sh.rustup.rs -o "$installer"
  sh "$installer" -y --no-modify-path --profile minimal --default-toolchain 1.90.0
fi
rustup toolchain install 1.90.0 --profile minimal --component rustfmt --component clippy --target wasm32-unknown-unknown
if ! command -v wasm-bindgen >/dev/null || [[ "$(wasm-bindgen --version)" != 'wasm-bindgen 0.2.105' ]]; then
  cargo install wasm-bindgen-cli --version 0.2.105 --locked --root /workspace/.diagram-tools/wasm-bindgen
fi
cargo fetch --locked
cargo build --locked --release -p diagram-cli
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
scripts/build-wasm.sh
# Optional browser/reference test dependencies. They are not CLI/WASM runtime deps.
npm ci --cache /workspace/.diagram-tools/npm --ignore-scripts --no-audit --no-fund
