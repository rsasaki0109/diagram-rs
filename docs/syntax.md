# Supported syntax by family

This is a tested Mermaid subset. Unknown constructs return source-located errors;
Mermaid's complete syntax, layout, formatting and browser features are not claimed.
The upstream development reference is pinned to Mermaid 11.12.0. Native, CLI and
WASM share the same parser and specialized renderers.

## Flowchart

`flowchart` or `graph` with TB/TD, BT, LR or RL. Seven shapes: rectangle `[]`,
rounded `()`, diamond `{}`, circle `(())`, stadium `([])`, subroutine `[[]]`,
cylinder `[()]`. Node IDs use ASCII letters, digits and underscores; labels can
be Japanese. Chains, fanout, labels, named classes, safe styles, JSON theme/spacing
config, cycles, self-loops and nested subgraphs are supported. Nesting is bounded
at 32; direction remains global. Source link lengths clamp at ten ranks as in
Mermaid. Bidirectional triangle arrowheads work with solid/dotted/thick links.

Unsupported: other shapes, circle/cross/source-only arrowheads, subgraph endpoints
and local directions, edge IDs, HTML/Markdown label formatting and interactions.
Parallel and opposite links use distinct routing channels where possible; very
dense layouts can still share segments or cross. A route or label that cannot be
placed safely returns an error. Layout is heuristic, not optimal.

## Sequence

`sequenceDiagram`; `participant ID as Label` and `actor ID as Label`; implicit
participants on messages. Arrows `->>`, `-->>`, `->`, `-->`, `-x`, `--x`; self
messages; bare `autonumber`. `activate ID`, `deactivate ID`, `->>+Target` and
`-->>-Target` activation shortcuts. Activations must be balanced.

`Note left of ID: text`, `Note right of ID: text`, `Note over ID[,ID]: text`.
Fragments `loop`, `alt`/`else`, `opt`, `par`/`and`, `critical`/`option`, `break`,
closed with `end`, nested up to 32. `<br>` and quoted `\n` produce multiline labels.

Unsupported: actor boxes, participant creation/destruction, JSON participant types,
rect backgrounds, bidirectional/async sequence arrows, custom autonumber ranges,
links and interactions. A branch must match its enclosing fragment.

Example: [sequence.mmd](../tests/fixtures/sequence.mmd).

## Class

`classDiagram`; global `direction`; `class ID` or `class ID["Label"]`; multiline
class bodies `{` ... `}` with one member per statement; `ID : member` outside a
body. Parentheses distinguish methods from attributes. Visibility and generic
notation remain literal text. `<<interface>>` and other annotation text inside a
class body appear above its title. Cards have separate title, attribute and method
compartments, including empty ones.

Relationships: `<|--`, `--|>`, `<|..`, `..|>`, `*--`, `--*`, `o--`, `--o`, `<--`,
`-->`, `<..`, `..>`, `--`, `..`. Optional quoted cardinalities at either endpoint;
optional `: label`. Dashed links without arrowheads remain dashed links.

Unsupported: namespaces, styles, interactions and single-line `{ members }`
bodies. A class's combined display label is limited to 4 KiB/32 lines; cardinality
labels to 32 bytes. Long/dense relationship labels can still cross other links.

Example: [class.mmd](../tests/fixtures/class.mmd).

## State

`stateDiagram` or `stateDiagram-v2`; global direction; `state ID`, `state "Label"
as ID`; `ID : description`; `A --> B : label`; `[*]` initial/final pseudostates.
`state ID <<choice>>`, `<<fork>>` and `<<join>>` stereotypes. Regular states retain
aliases and descriptions; choice diamonds, bars and distinct start/end symbols
use their own geometry. Cycles and self transitions are supported.

Unsupported: composite states, concurrency separators, notes, classes/styles and
history pseudostates. `__initial` and `__final` are reserved internal IDs.

Example: [state.mmd](../tests/fixtures/state.mmd).

## Entity relationship

`erDiagram`; global direction; entities and multiline attribute bodies. Attributes
are `type name [PK, FK, UK] ["comment"]`. Duplicate attribute names or keys are
errors. Entity cards contain measured table columns for type, name, keys and
optional comments.

Cardinality: exactly one `||`; optional one `|o` on the left / `o|` on the right;
one or more `}|` / `|{`; zero or more `}o` / `o{`. Relationships use identifying
`--` or non-identifying `..`, and `: label`. SVG uses corresponding crow's-foot
symbols at both ends.

Unsupported: natural-language cardinalities, aliases, styles and interactions.
Types support ASCII letters, digits, underscores and array brackets; IDs/names use
ASCII letters, digits and underscores. Comments and relationship labels can be
Japanese. An entity is limited to 256 attributes.

Example: [er.mmd](../tests/fixtures/er.mmd).

## Gantt

`gantt`; `title`, `section`, `dateFormat YYYY-MM-DD`, `axisFormat %Y-%m-%d`,
`todayMarker off`. Tasks: `Label : [status,] [id,] start, end/duration`. Status tags
`done`, `active`, `crit`, `milestone`; `done`/`active` select one primary status.
Calendar dates use four-digit Gregorian years and validated leap days. Durations
are whole days `d` or weeks `w`; end dates are exclusive. Milestones allow zero
duration. `after task_id [task_id ...]` uses the latest predecessor end; forward
references are resolved and cycles/missing dependencies are diagnosed.

An omitted start continues after the previous source task; the first task must
have an explicit start date. There is no dependency on the machine's current date
or timezone. Rows retain source/section order; Mermaid may sort rows by date.
Date ranges are bounded to 100 years; task/dependency counts are limited.

Unsupported: working-day exclusions, hours/minutes, custom date/axis formats,
interactions and current-date markers. The default native chart does not show a
current-date marker. Dependency lines can cross in dense schedules.

Example: [gantt.mmd](../tests/fixtures/gantt.mmd).

## Shared behavior and limits

128 KiB UTF-8 source; 512 nodes/classes/entities/states/participants/tasks;
2,048 edges/events/relationships/dependencies; 4 KiB/32-line text fields; flowchart
layout expansion at most 8,192 vertices. Unknown init keys/styles are errors.
Colors cannot load external resources. Supported shared config is strict JSON
`theme` and `flowchart.nodeSpacing`/`rankSpacing` (used for graph-based families).
Sequence and Gantt have specialized measured timelines.

The default `cjk` feature bundles DejaVu Sans and Noto Sans JP fallback. Japanese,
Latin and many other covered scripts work without installed fonts. Unsupported
characters return an error; custom fonts replace the default collection. Complete
paragraph bidi, emoji fallback, Markdown and HTML rendering remain unsupported.
See [font provenance and licenses](../assets/fonts/README.md).
