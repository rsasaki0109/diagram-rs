# Architecture

```
source -> diagram-parser -> diagram-core::Graph
                                  |
                         diagram-layout + font metrics
                                  |
                          diagram-render -> SVG
                                  |
                    diagram-cli / diagram-wasm
```

`diagram-core` owns stable vector indices, source spans, styles, shapes, groups,
configuration and IR validation. It serializes with Serde, allowing other native
front ends to construct diagrams. Indices remain unchanged by layout.
`diagram-parser` produces this typed representation or a source-located error.
`diagram-layout` only needs the IR and a `TextMeasure` implementation; geometry,
ranks, crossing statistics and routes can be inspected independently.
`diagram-render` implements measurement and glyph paths using a font collection, and can
render precomputed geometry. It does not depend on the parser. The CLI and WASM
compose these crates at their public boundaries.

No service, database, browser, Node.js, or network is needed for normal native
rendering. Browser integration calls the same parser/layout/renderer compiled to
WASM. JavaScript handles module initialization and passing strings/bytes only.
`Diagram` dispatches six implemented families. Flowcharts, class/state relations
and ER entities share hierarchical routing; class compartments and ER table sizes
supply measured minimum boxes. Sequence and Gantt have their own timeline layouts.
Each family retains its own typed IR, validation and dedicated rendering.
`parse_diagram`/`render_diagram` are the shared CLI/WASM entry points; original
flowchart APIs remain available.

## Constraints and quality checks

- 128 KiB source, 512 nodes, 2,048 edges, 4 KiB / 32-line labels.
- Layout expansion limit: 8,192 vertices; edge rank span: 1–32; spacing: 24–300 px.
- Font size: 1–48 px; stroke width: 1–8 px.
- Parser rejects unknown statements, CSS properties, classes and config keys.
- Geometry tests exercise all four directions, compound groups, feedback edges,
  labels, disconnected nodes and seeded generated graphs.
- Tests measure node overlap, routes through unrelated nodes, group containment,
  bounds, crossing reduction, and label placement.
- Raster and SVG regression fixtures pin deterministic rendering. Rasterization
  via resvg is a test tool, not part of the output pipeline.

Dense or parallel edges can share channels; this implementation does not minimize
all edge/edge or edge/label crossings. Glyphs are paths with accessible titles,
not selectable `<text>`. Unsupported glyphs return an error. These limitations
are explicit rather than replaced with browser rendering or guessed geometry.

Edge target (`directed`) and source (`source_arrow`) arrowheads are independent
from feedback reversal during layout. `min_rank_span` constrains DAG ranks after
cycle handling; extra dummy vertices carry long routes across intermediate layers.
Parsed link lengths clamp to ten ranks, matching Mermaid 11.12.0; direct IR
can request up to 32. Self-loops have no rank constraint. Older serialized edges default to a span of
one and no source arrow. Nested group frames expand around descendant frames, preserving separate sibling
lanes and title clearance. Stadium, subroutine and cylinder sizes reserve space for
curved ends, internal borders and ellipse caps before placing measured text.

Default font shaping picks DejaVu Sans first and falls back to Noto Sans JP for
uncovered characters. Glyph IDs encode the selected font; outlines and advances
use the same normalized units. Unchanged Latin labels retain their original SVG.
The `cjk` feature can be disabled on CLI/WASM builds to reduce artifact size.
