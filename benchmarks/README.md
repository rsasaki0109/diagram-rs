# Measured performance

These are local measurements of the six-family implementation with default
Japanese fallback on **2026-10-11**, not comparative speedup claims or release
guarantees. Raw data is in [measurements.csv](measurements.csv) and
[environment.json](environment.json). Only ASCII flowchart workloads are timed;
these numbers do not characterize sequence/class/state/ER/Gantt or Japanese shaping.

Environment: Linux x86_64, glibc 2.41, Intel Xeon Platinum 8370C, five available
CPUs, Rust 1.90.0. Rendering is single-threaded. Release profile uses thin LTO,
one codegen unit and stripped binaries. No builds or browser tests ran concurrently
with measurement; cloud scheduling and CPU frequency still introduce variability.
The source is the local, uncommitted workspace. Lockfile and input hashes are
recorded in the JSON. Previous initial measurements used a different reported CPU;
do not infer a speedup from the difference.

## Stage timings

Values are microseconds per render operation: the **median of five batch means**
for each independent stage, with ten warm-up iterations before each stage. The
iteration column is the count per batch. These are neither percentile latencies
nor confidence intervals. Stages are timed separately and do not necessarily add
up to the end-to-end timing because of independent scheduling/cache conditions.

| Nodes | Edges | Iterations/batch | Parse (µs) | Layout (µs) | SVG (µs) | End-to-end (µs) |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 8 | 8 | 500 | 12.53 | 60.15 | 481.36 | 607.10 |
| 64 | 78 | 100 | 177.66 | 563.12 | 2368.03 | 3192.20 |
| 256 | 318 | 20 | 1665.06 | 3061.04 | 8626.66 | 13763.15 |

Inputs are small (8), medium (64), and large (256) labeled LR graphs: a chain plus
skip edges every fourth node. They exercise long-edge dummy vertices, routing,
text measurement and SVG output. They do not represent every flowchart topology;
dense cycles/compound graphs can behave differently. Parsing constructs validated
IR. Layout includes shaping for measurement. SVG timing uses precomputed geometry
but includes glyph shaping/outlines, escaping and deterministic fragment namespaces.
End-to-end includes parsing, bundled-font construction, layout and SVG generation,
without CLI startup/file I/O. `black_box` prevents unused results from being removed.

## Memory and artifact sizes

Peak RSS is read from Linux `/proc/self/status` (`VmHWM`) in a **fresh native Rust
benchmark process performing one end-to-end render**, one process per size. It
includes the process, bundled font and all pipeline allocations; it is not isolated
algorithm memory or a CLI peak-RSS measurement. No allocator-level/WASM-memory
measurement is claimed.

| Nodes | Process peak RSS (KiB) |
| ---: | ---: |
| 8 | 3344 |
| 64 | 3576 |
| 256 | 4856 |

| Artifact | Bytes | gzip bytes |
| --- | ---: | ---: |
| native | 12003568 | 6971140 |
| wasm | 11474353 | 6652897 |

The generated browser package directory is 11673818 bytes including
JavaScript, declarations, WASM, README and both font license notices.
The complete, unmodified Japanese font substantially increases artifact size;
A separate Latin-only native build is 2405072 bytes (1114580
gzip bytes). Its ASCII fixture matches the default build; Japanese returns an
error unless a custom font is supplied. Its timings/RSS and smaller WASM builds
were not measured. WASM numbers are after
wasm-bindgen 0.2.105, without wasm-opt; gzip uses Python's default level and zero
timestamp. The standalone native binary bundles both fonts and still uses system
libc. Include the project/dependency/font notices when redistributing it.

## Reproduce

With Rust and wasm-bindgen activated, from the repository root:

```sh
scripts/build-wasm.sh
python3 benchmarks/run.py
```

The separate smaller native build can be reproduced with
`cargo build --release --locked -p diagram-cli --no-default-features
--target-dir /tmp/diagram-rs-latin-target`; its size is recorded under
`additional_builds` in the reviewed JSON. The script below does not measure it.

The script builds the release CLI and timing harness, measures only after those
builds finish, validates exit statuses, records fresh-process RSS, and writes raw
results and input diagrams under the ignored `benchmarks/results/` directory.
It does not overwrite these reviewed measurements automatically. Review changes
before copying results to `measurements.csv` and `environment.json`. Memory
measurement currently requires Linux. Browser, Node.js and network access are not
required once the dependencies/toolchain are installed. Performance is not a CI
threshold; correctness and the supported compatibility surface take priority.
