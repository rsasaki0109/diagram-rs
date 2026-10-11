# Builds, installation and releases

Version 0.1.0 is a local development version. No registry package or GitHub release
has been published. The Linux GNU native binary and browser ESM package have been
built and exercised locally. macOS/Windows jobs are configured in CI but their
results are unverified; do not describe those platforms as release-tested yet.

## Build and install locally

With Rust 1.90.0 and the locked dependencies, run `cargo install --locked --path
crates/diagram-cli`. The executable is `diagram-rs`. A default build embeds DejaVu
Sans and Noto Sans JP; use `--no-default-features` for the smaller Latin-only CLI.
The smaller build rejects Japanese unless a custom font is supplied. See
[syntax.md](syntax.md) for the six supported diagram families and limitations.

For browser ESM, install wasm-bindgen-cli 0.2.105, run `scripts/build-wasm.sh`, and
use the generated `crates/diagram-wasm/pkg` directory. Serve its WASM asset beside
the JavaScript files. TypeScript declarations and a real esbuild integration are
validated by `npm run test:bundle`. Other bundlers and Node.js rendering are not
claimed. To install the local tarball in a separate application, use
`npm install /absolute/path/to/dist/diagram-rs-0.1.0.tgz` after preparing it below.

## Prepare reviewable artifacts

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
npm ci --ignore-scripts
npm run test:browser
npm run test:reference
npm run test:bundle
scripts/release-local.sh
```

The release script builds binaries and bindings, creates a source archive, a
Linux GNU native archive and a local npm tarball, then verifies the CLI extracted
from the native archive against every fixture. It checks the npm entrypoints and
both font licenses, and writes `dist/manifest.json` and `SHA256SUMS`. These artifacts
are unsigned and unpublished. Native/source tar metadata and gzip timestamps are
normalized; compiler bit-for-bit reproducibility across machines is not claimed.
The native executable uses the build host's system libc; it is not a static musl
binary or a promise of compatibility with older Linux distributions.

## Maintainer release review

1. Confirm the version in Cargo and npm metadata agrees and record the source
   commit. Review syntax exceptions, API changes and updated measurements.
2. Require passing remote Linux/macOS/Windows native and Linux browser jobs.
   Inspect visual regressions; validate the archives and license inventory.
3. Review artifact checksums and installation instructions. Rust path dependencies
   currently support workspace/local installation; registry publication requires
   adding versioned dependency constraints and verifying package ordering first.
4. Obtain maintainer authorization for tags, pushes and registry/GitHub publication.
   Publish only the exact reviewed version and keep checksums with the release.
5. Record release notes, supported platforms, limitations and any security fixes.

Rust structures and syntax may change before 1.0. Preserve the existing flowchart
API where practical, document incompatible changes and test both native and WASM
entrypoints. Unsupported syntax must produce an error instead of silently changing
the diagram. No long-term support or production reliability guarantee is implied
by this initial version.
