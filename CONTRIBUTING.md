# Contributing

Use Rust 1.90.0 (pinned in `rust-toolchain.toml`). Do not create a second checkout
or worktree for ordinary development in an isolated cloud task.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked -p diagram-cli
```

Rendering changes must update meaningful geometry invariants and be visually
reviewed. To regenerate baselines intentionally:

```sh
cargo run --locked -p diagram-render --example snapshots
```

Inspect all changed SVG/PNG files under `tests/snapshots/` and `assets/hero.svg`.
Do not regenerate snapshots merely to hide a failure. Tests compare pinned resvg
raster output exactly; dependency upgrades should include baseline review.

For browser tests, install wasm-bindgen-cli 0.2.105 and optional dev tools:

```sh
cargo install wasm-bindgen-cli --version 0.2.105 --locked
scripts/build-wasm.sh
npm ci --ignore-scripts
npx playwright install --with-deps chromium
npm run test:browser
npm run test:reference
npm run test:bundle
```

Set `DIAGRAM_CHROMIUM=/path/to/chromium` to use an existing browser. Node/browser
packages are development dependencies only. Keep unknown syntax errors explicit,
include source positions, and avoid dependencies unless their benefit is concrete.
Every added diagram type needs parser/layout/rendering/test coverage before it is
listed as supported. Never publish crates/npm packages as part of a contribution
without maintainer approval.

After changing Rust dependencies, regenerate `THIRD_PARTY_LICENSES.txt` using
`python3 scripts/collect-licenses.py` and review upstream licenses. Native binary
archives must include this file, the project license, and both font licenses.
Use `scripts/release-local.sh` to prepare and verify local artifacts; see
`docs/releases.md` for the review and publication process.
