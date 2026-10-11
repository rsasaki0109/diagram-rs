# Maturity assessment and target

Last assessed: 2026-10-11. Current score: **80%**. Accepted first target: **80%**.

This is a project planning score, not a measured percentage of Mermaid syntax,
an upstream rating, or proof of production reliability. Scores are provisional
engineering judgments. Keep the rubric fixed and change scores only when there
is new evidence. Report this score at the end of development updates.

## Comparison scope

Mermaid.js is the primary reference for an established diagram product. Compare
six diagram families: flowchart, sequence, class, state, ER and Gantt, together
with rendering quality, integration, distribution and documentation. A score of
100 means established OSS quality within this scope; it does not mean support
for every Mermaid diagram family, configuration option or browser interaction.

The Rust reference is mermaid-rs-renderer 0.3.1, whose published README advertises
23 diagram families. Its feature breadth is ahead of this project, but we have
not run a comparable correctness or reliability evaluation of that implementation.
Do not assign it an invented numerical rating or infer quality from family count.
mermaid-rs 0.1.1 wraps Mermaid JS and is not a native-engine equivalent. See
[research.md](research.md) for the inspected versions and sources.

## Fixed scoring rubric

The total is the sum of weight multiplied by category score, divided by 100.
Category scores are estimates; the resulting total is a planning indicator.

| Category | Weight | Current category score | Evidence and remaining work |
| --- | ---: | ---: | --- |
| Features and compatibility | 40% | 70% | Practical subsets of all six families; Japanese, nested groups and distinct parallel routes; 149 upstream structural cases agree. Composite states, working-day calendars and broader syntax remain absent |
| Rendering quality and reliability | 25% | 82% | 67 native tests, SVG/PNG and geometry regressions, native/WASM parity and mutation campaigns pass; revised remote Linux/macOS/Windows and browser jobs all pass. Sustained coverage-guided fuzzing and release history remain absent |
| APIs and integration | 15% | 90% | Rust/CLI verified on Linux/macOS/Windows; standard/Latin-only browser WASM, TypeScript and esbuild pass local and remote checks. Long-term API stability and other bundlers remain unverified |
| Distribution and operations | 10% | 85% | Versioned native/source/npm archives, clean source revision, extracted CLI parity, SHA256 inventory and licenses verified; remote CI builds downloadable artifacts. Registry/GitHub releases remain unpublished; prebuilt native archives cover Linux GNU only |
| Documentation and maintenance | 10% | 90% | Family syntax, architecture, examples, measured sizes/timings, tested installation/integration and release guides include actual CI evidence. Real maintenance/support history remains absent |

Calculation: `40*70/100 + 25*82/100 + 15*90/100 + 10*85/100 + 10*90/100 = 79.5 ≈ 80`.
Passing a few more tests does not by itself justify a large maturity increase.

## Accepted 80% milestone

- Implement a documented practical subset of all six core diagram families.
- Broaden flowchart support, including nested groups and common shapes/arrows.
- Improve dense routing and multilingual font handling, including Japanese.
- Exercise each family with upstream comparisons, visual and geometry regressions,
  invalid inputs and fuzzing; document compatibility exceptions.
- Verify Rust, CLI and browser WASM across supported platforms and representative
  integrations; demonstrate reproducible builds and remote CI.
- Prepare and validate versioned packages and release artifacts, installation
  guides, license notices and a release process. Publication requires appropriate
  authorization; recommending this target does not authorize publication.

Before assigning 80%, review evidence against every category; implementing six
parsers alone is insufficient. Adoption and maintenance history remain external
evidence that a single development session cannot manufacture.

Suggested checkpoints: **50%** for a stronger flowchart engine plus sequence
diagrams; **65%** for practical support of the six families with regression
coverage; **80%** for validated integration, distribution and release readiness.
These are review checkpoints, not automatic scores for completing a checklist.

Future update footer: `成熟度：80%／目標80%（主要6種・暫定評価）`.

Assessment history: 30% before sequence support; 34% after verified sequence
parsing, specialized geometry, CLI/WASM dispatch, visual regressions and upstream
participant/message comparisons. Class work was in progress at that checkpoint.

53%: six diagram-family pipelines, Japanese fallback, nested groups and structural
reference checks verified locally. GitHub API access was denied by the environment
network policy; no remote CI result or release publication is claimed.


72%: all local checks above passed, including real browser bundling and extracted
release-binary verification against 15 fixtures. Current native/WASM sizes and
flowchart timings were remeasured with Japanese fallback enabled. Linux/macOS/
Windows native jobs are configured, but only local Linux is verified. After the
environment configuration was published, GitHub API access and the initial source
push succeeded. The [first remote CI run](https://github.com/rsasaki0109/diagram-rs/actions/runs/38112746456)
is queued; results are not yet verified. A source-archive build in a separate
directory also produces matching Japanese SVG, but native binary hashes differ,
so bit-for-bit build reproducibility is not claimed. The 80% milestone is
**not achieved**.


76%: initial remote Linux, macOS and browser jobs passed; Windows failed in the
native tests. A Windows-style CRLF checkout reproduces the SVG baseline failure;
with the new LF attributes an isolated checkout passes the same SVG/PNG test.
The [revised PR CI](https://github.com/rsasaki0109/diagram-rs/actions/runs/38113460459)
is still pending, so Windows is not yet counted as verified. The Latin-only WASM
variant passes six-family browser parity and external Japanese-font rendering.
The source-archive rebuild matches all 16 SVG cases, and release manifests record
a clean source revision. The 80% milestone remains **not achieved**.


80%: the [revised CI run](https://github.com/rsasaki0109/diagram-rs/actions/runs/38113460459)
for engine/release commit `ea93a13082ce450b812fdcb77787c7d34b0c758d` completed
successfully: native Linux, macOS and Windows tests/builds/rustdoc plus browser
standard/Latin-only WASM, 149 Mermaid comparisons, TypeScript/esbuild and release
archive verification. The Windows newline regression is fixed by Git attributes.
This meets the accepted practical six-family milestone; it is not full Mermaid
compatibility or proof of production reliability. The later assessment update is
documentation only. Changes are reviewable in [PR #1](https://github.com/rsasaki0109/diagram-rs/pull/1);
no merge, registry publication or GitHub release publication is claimed.
