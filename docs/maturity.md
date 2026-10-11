# Maturity assessment and target

Last assessed: 2026-10-11. Current score: **72%**. Accepted first target: **80%**.

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
| Rendering quality and reliability | 25% | 75% | 67 native tests, SVG/PNG and geometry regressions, 16 native/WASM fixtures, 6,000 parser and 720 renderer mutation inputs pass locally. Dense routing is heuristic; sustained fuzzing and release history remain absent |
| APIs and integration | 15% | 80% | Rust, CLI and browser WASM; strict TypeScript and real esbuild bundle verified. Remote macOS/Windows results and long-term API stability remain unverified |
| Distribution and operations | 10% | 50% | Versioned native/source/npm archives, extracted CLI checks, SHA256 inventory and licenses verified. Packages unpublished; remote CI unverified |
| Documentation and maintenance | 10% | 85% | Family syntax, architecture, examples, current measurements, integration and release guides exist. Real maintenance/support history remains absent |

Calculation: `40*70/100 + 25*75/100 + 15*80/100 + 10*50/100 + 10*85/100 = 72.25 ≈ 72`.
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

Future update footer: `成熟度：72%／目標80%（主要6種・暫定評価）`.

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
