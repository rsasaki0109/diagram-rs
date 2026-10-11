# Research and design decisions

Investigated on 2026-10-11. This document separates upstream claims from what
this project implements. No implementation source was copied from these projects.

## Mermaid as a behavioral reference

Mermaid parses diagram-specific languages into database/IR-like structures,
selects a diagram renderer and layout, measures labels in a browser/DOM, and
emits SVG. Its flowchart grammar is maintained as Jison source. The application
supports far more than flowcharts: sequence, class, state, ER, Gantt, and other
families have their own parsers and renderers. Flowcharts include compound groups,
many shapes, links, styles, configuration, HTML/Markdown labels and interactions.
The existence of these features upstream does not imply support here.

Sources inspected:

- [Flowchart documentation](https://mermaid.js.org/syntax/flowchart.html)
  and [source](https://github.com/mermaid-js/mermaid/blob/develop/packages/mermaid/src/docs/syntax/flowchart.md).
- [Native grammar reference](https://github.com/mermaid-js/mermaid/blob/develop/packages/mermaid/src/diagrams/flowchart/parser/flow.jison).
- [Package manifest](https://github.com/mermaid-js/mermaid/blob/develop/packages/mermaid/package.json),
  including Dagre and optional layout integrations.
- [Mermaid license](https://github.com/mermaid-js/mermaid/blob/develop/LICENSE): MIT.

The moving `develop` documentation also describes newer defaults (including ELK).
Compatibility tests instead pin **Mermaid 11.12.0** from npm. A browser test asks
that version to parse and render eight shared fixtures and seventeen connector cases and compares node IDs,
shapes, endpoints, arrowheads, line styles and rank spans, in addition to
edge counts. Its DB label representations are retained for manual inspection.
This is a structural sample, not proof of full syntax or visual parity. The SVG
styles and coordinates are deliberately not expected to match pixel for pixel.
Mermaid is an optional **development reference**, never a runtime dependency of
our Rust crates, executable, or generated WASM package.

## Existing Rust projects

Metadata and published source archives were read from crates.io. Archive SHA-256
checksums were verified against crates.io metadata before inspection.

| Project | Observed scope | License | Consequence for diagram-rs |
| --- | --- | --- | --- |
| [mermaid-rs 0.1.1](https://crates.io/crates/mermaid-rs) | Rust bindings to Mermaid JS | MIT | Does not meet the requested native pipeline |
| [mermaid-rs-renderer 0.3.1](https://crates.io/crates/mermaid-rs-renderer) | Native parser/layout/SVG; its README advertises 23 diagram families and large speedups | MIT | Native Rust is already an established approach; we do not repeat its performance claims |
| [oxdraw 0.2.1](https://crates.io/crates/oxdraw) | Declarative diagrams, CLI and web editing, according to published package metadata | MIT | Interactive editing is a different focus; no compatibility/quality ranking was inferred |
| [layout-rs 0.1.3](https://crates.io/crates/layout-rs) | Directed graph visualization, DOT examples and SVG | MIT | Useful algorithm reference; not used as the engine |
| [petgraph 0.8.3](https://crates.io/crates/petgraph) | General graph structures/algorithms | MIT OR Apache-2.0 | Stable vectors suffice for this bounded flowchart IR; avoid another production dependency |

The differentiation is a deliberately small, inspectable compatibility surface,
source diagnostics, deterministic native/browser parity, explicit rejection of
unsupported syntax, and font outlines measured and emitted from the same font.
It is not a claim to be the first, fastest, or most compatible Rust implementation.

## Layout references and algorithm selection

- Sugiyama, Tagawa, Toda (1981), *Methods for Visual Understanding of Hierarchical
  System Structures*, IEEE TSMC, DOI: [10.1109/TSMC.1981.4308636](https://doi.org/10.1109/TSMC.1981.4308636).
- Gansner et al. (1993), *A Technique for Drawing Directed Graphs*, IEEE TSE,
  DOI: [10.1109/32.221135](https://doi.org/10.1109/32.221135).
- [Dagre](https://github.com/dagrejs/dagre), MIT, a client-side directed-graph
  layout implementation and useful related work.

The Rust implementation is a Sugiyama-style pipeline, written here: DFS feedback
edge reversal for ranking, longest-path ranks in the resulting DAG, dummy vertices
on long edges, twelve alternating barycenter sweeps, retention of the best crossing
count, rank centering with measured dimensions, and orthogonal inter-rank channels.
Reversed edges retain their source/target and arrow direction in the IR/output.
This is not Dagre's network-simplex or Brandes–Köpf implementation. Crossing
minimization is heuristic, not optimal. Flat subgraphs reserve disjoint lanes and
trade compactness for predictable boundaries. Self-loops have local side routes.

## Parsing, typography and SVG choices

A purpose-built scanner preserves UTF-8 byte offsets and Unicode-aware columns.
A small recursive statement parser provides clearer unsupported-feature errors
than accepting the complete Mermaid grammar with incomplete semantics. There is
no generated parser dependency. This is not a lossless full-Mermaid AST: the typed
IR records resolved labels, styles and spans; comments/classes are normalized.

[ttf-parser](https://crates.io/crates/ttf-parser) (MIT OR Apache-2.0) supplies safe
font parsing through [rustybuzz](https://crates.io/crates/rustybuzz) (MIT), a native
HarfBuzz shaping implementation. Widths include shaped advances and glyph bounds;
line height uses ascent/descent/line gap. The same glyphs become SVG paths with
reused definitions. This costs a bundled font and selectable SVG text, but avoids
font substitution and machine-specific text bounds. Complex bidirectional mixed
paragraphs and emoji fallback are not promised. The current implementation
adds Noto Sans JP fallback for Japanese (see `assets/fonts/README.md`). A custom font
API/CLI option covers additional glyph sets.

The renderer writes XML directly from validated IR and escaped attributes. Styles
are restricted to safe literal colors and bounded numeric widths/sizes. There are
no external resources, callbacks, scripts, HTML labels, or raw CSS. Resource limits
bound parsing and expanded layout work. A strict browser CSP must permit WASM
execution separately; no special CSP override is bundled.

## Licensing

Project code is MIT. The original bundled DejaVu Sans binary is unmodified and
uses the Bitstream Vera notice (DejaVu changes are public domain). Its full license
is shipped under `assets/fonts/`, copied into the WASM package, and retained in SVG
metadata when its outlines are emitted. User-supplied font licensing remains the
caller's responsibility. Rust production dependencies use MIT, Apache-2.0, or
Unicode licenses; dev dependencies and reference packages are separate. `THIRD_PARTY_LICENSES.txt` retains upstream Rust notices (including dev-only
crates conservatively) and is included in the generated WASM package. The
lockfiles record reproducible resolutions. No incompatible code was incorporated.
