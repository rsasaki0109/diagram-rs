//! Typed, serializable intermediate representation shared by the diagram pipeline.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt};

/// Input limits bound parser and layout resource use, including in a browser.
pub const MAX_NODES: usize = 512;
/// Maximum edges in a diagram.
pub const MAX_EDGES: usize = 2048;
/// Maximum UTF-8 source bytes.
pub const MAX_SOURCE_BYTES: usize = 128 * 1024;

/// Source position. Offsets and lengths are bytes; line and column are one-based.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub offset: usize,
    pub len: usize,
    pub line: usize,
    pub column: usize,
}
/// An error with an actionable message and source position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}
impl Diagnostic {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {}",
            self.span.line, self.span.column, self.message
        )
    }
}
impl std::error::Error for Diagnostic {}

/// Direction of the primary flow axis. TD is parsed as TB.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    #[default]
    TB,
    BT,
    LR,
    RL,
}
/// Supported flowchart node geometries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shape {
    #[default]
    Rectangle,
    Rounded,
    Diamond,
    Circle,
    Stadium,
    Subroutine,
    Cylinder,
    Initial,
    Final,
    Bar,
}
/// Line treatment of an edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineKind {
    #[default]
    Solid,
    Dotted,
    Thick,
}
/// A small, deliberately safe subset of Mermaid CSS styling.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub color: Option<String>,
    pub stroke_width: Option<f64>,
    pub font_size: Option<f64>,
}
impl Style {
    /// Overlay specified fields only, preserving unspecified values.
    pub fn overlay(&mut self, other: &Self) {
        if other.fill.is_some() {
            self.fill.clone_from(&other.fill);
        }
        if other.stroke.is_some() {
            self.stroke.clone_from(&other.stroke);
        }
        if other.color.is_some() {
            self.color.clone_from(&other.color);
        }
        if other.stroke_width.is_some() {
            self.stroke_width = other.stroke_width;
        }
        if other.font_size.is_some() {
            self.font_size = other.font_size;
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        for value in [&self.fill, &self.stroke, &self.color]
            .into_iter()
            .flatten()
        {
            if !safe_color(value) {
                return Err(format!("unsafe or unsupported color: {value}"));
            }
        }
        for (name, value, max) in [
            ("stroke-width", self.stroke_width, 8.0),
            ("font-size", self.font_size, 48.0),
        ] {
            if let Some(v) = value
                && (!v.is_finite() || v < 1.0 || v > max)
            {
                return Err(format!("{name} must be between 1 and {max}"));
            }
        }
        Ok(())
    }
}
/// Colors cannot reference external URLs, CSS variables, or executable content.
pub fn safe_color(s: &str) -> bool {
    let hex = s.strip_prefix('#').is_some_and(|s| {
        matches!(s.len(), 3 | 4 | 6 | 8) && s.bytes().all(|b| b.is_ascii_hexdigit())
    });
    hex || matches!(
        s,
        "none"
            | "transparent"
            | "black"
            | "white"
            | "red"
            | "green"
            | "blue"
            | "gray"
            | "grey"
            | "orange"
            | "purple"
            | "yellow"
            | "pink"
            | "cyan"
            | "navy"
            | "teal"
            | "brown"
    )
}
/// Node IDs come from source, while edges have stable insertion-order indices.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub shape: Shape,
    pub style: Style,
    pub group: Option<usize>,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub source: usize,
    pub target: usize,
    pub label: String,
    pub directed: bool,
    /// Arrow at the source endpoint, independently of the target arrow.
    #[serde(default)]
    pub source_arrow: bool,
    /// Minimum rank difference for a non-self edge (1..=32).
    #[serde(default = "default_rank_span")]
    pub min_rank_span: usize,
    pub kind: LineKind,
    pub style: Style,
    pub span: Span,
}
fn default_rank_span() -> usize {
    1
}
/// Compound grouping. Parents precede children; depth is limited to 32.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Group {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<usize>,
    pub id: String,
    pub label: String,
    pub span: Span,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}
/// Render and layout configuration parsed from a supported init directive.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub theme: Theme,
    pub node_spacing: f64,
    pub rank_spacing: f64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::Light,
            node_spacing: 40.0,
            rank_spacing: 70.0,
        }
    }
}
/// Flowchart IR. Vector order is deterministic and preserved across the pipeline.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub direction: Direction,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub groups: Vec<Group>,
    pub config: Config,
}
impl Graph {
    /// Check invariants even for graphs constructed directly or deserialized.
    pub fn validate(&self) -> Result<(), Diagnostic> {
        let fail = |s| {
            Diagnostic::new(
                s,
                Span {
                    line: 1,
                    column: 1,
                    ..Span::default()
                },
            )
        };
        if self.nodes.is_empty() {
            return Err(fail("diagram contains no nodes"));
        }
        if self.nodes.len() > MAX_NODES
            || self.edges.len() > MAX_EDGES
            || self.groups.len() > MAX_NODES
        {
            return Err(fail("diagram exceeds node, edge, or group limits"));
        }
        for s in [self.config.node_spacing, self.config.rank_spacing] {
            if !s.is_finite() || !(24.0..=300.0).contains(&s) {
                return Err(fail("spacing must be between 24 and 300"));
            }
        }
        let mut ids = BTreeSet::new();
        for n in &self.nodes {
            if n.id.is_empty() || !ids.insert(&n.id) {
                return Err(Diagnostic::new("empty or duplicate node id", n.span));
            }
            if n.group.is_some_and(|g| g >= self.groups.len()) {
                return Err(Diagnostic::new("invalid group index", n.span));
            }
            n.style.validate().map_err(|e| Diagnostic::new(e, n.span))?;
            validate_text(&n.label, n.span)?;
            validate_text(&n.id, n.span)?;
        }
        for e in &self.edges {
            if !(1..=32).contains(&e.min_rank_span) {
                return Err(Diagnostic::new(
                    "edge rank span must be between 1 and 32",
                    e.span,
                ));
            }
            if e.source >= self.nodes.len() || e.target >= self.nodes.len() {
                return Err(Diagnostic::new("invalid edge endpoint", e.span));
            }
            e.style
                .validate()
                .map_err(|e2| Diagnostic::new(e2, e.span))?;
            validate_text(&e.label, e.span)?;
        }
        let mut group_ids = BTreeSet::new();
        for (i, g) in self.groups.iter().enumerate() {
            if g.parent.is_some_and(|p| p >= i) {
                return Err(Diagnostic::new(
                    "group parent must precede its child",
                    g.span,
                ));
            }
            let mut parent = g.parent;
            let mut depth = 1;
            while let Some(p) = parent {
                depth += 1;
                if depth > 32 {
                    return Err(Diagnostic::new("subgraph depth exceeds 32", g.span));
                }
                parent = self.groups[p].parent;
            }
            if g.id.is_empty() || !group_ids.insert(&g.id) || ids.contains(&g.id) {
                return Err(Diagnostic::new(
                    "duplicate group id or group/node id collision",
                    g.span,
                ));
            }
            if !self.nodes.iter().any(|n| self.group_contains(n.group, i)) {
                return Err(Diagnostic::new("empty subgraph", g.span));
            }
            validate_text(&g.label, g.span)?;
            validate_text(&g.id, g.span)?;
        }
        Ok(())
    }
}
impl Graph {
    /// Whether a node's direct group belongs to the given group's subtree.
    pub fn group_contains(&self, mut group: Option<usize>, ancestor: usize) -> bool {
        for _ in 0..32 {
            let Some(g) = group else {
                return false;
            };
            if g == ancestor {
                return true;
            }
            let Some(g) = self.groups.get(g) else {
                return false;
            };
            group = g.parent;
        }
        false
    }
    /// Parent-before-child traversal for layout lanes; sibling order is stable.
    pub fn group_order(&self) -> Vec<usize> {
        fn walk(g: usize, groups: &[Group], out: &mut Vec<usize>) {
            out.push(g);
            for (i, c) in groups.iter().enumerate() {
                if c.parent == Some(g) {
                    walk(i, groups, out);
                }
            }
        }
        let mut out = Vec::new();
        for (i, g) in self.groups.iter().enumerate() {
            if g.parent.is_none() {
                walk(i, &self.groups, &mut out);
            }
        }
        out
    }
}
pub(crate) fn validate_text(s: &str, span: Span) -> Result<(), Diagnostic> {
    if s.len() > 4096
        || s.split('\n').count() > 32
        || s.chars().any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(Diagnostic::new(
            "label exceeds limits or contains invalid control characters",
            span,
        ));
    }
    Ok(())
}
/// Text measurement shared with layout; implementations must use actual font metrics.
pub trait TextMeasure {
    fn measure(&self, text: &str, font_size: f64) -> Result<Size, String>;
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    pub fn center(self) -> Point {
        Point {
            x: self.x + self.width / 2.0,
            y: self.y + self.height / 2.0,
        }
    }
    pub fn overlaps(self, b: Self) -> bool {
        self.x < b.x + b.width
            && b.x < self.x + self.width
            && self.y < b.y + b.height
            && b.y < self.y + self.height
    }
}

/// Diagram families with implemented parsers, layouts and renderers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Diagram {
    Flowchart(Graph),
    Sequence(sequence::Sequence),
    Class(class::ClassDiagram),
    State(state::StateDiagram),
    Er(er::ErDiagram),
    Gantt(gantt::Gantt),
}
pub mod class;
pub mod er;
pub mod gantt;
pub mod sequence;
pub mod state;
impl Diagram {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        match self {
            Self::Flowchart(g) => g.validate(),
            Self::Sequence(s) => s.validate(),
            Self::Class(c) => c.validate(),
            Self::State(s) => s.validate(),
            Self::Er(e) => e.validate(),
            Self::Gantt(g) => g.validate(),
        }
    }
    pub fn config_mut(&mut self) -> &mut Config {
        match self {
            Self::Flowchart(g) => &mut g.config,
            Self::Sequence(s) => &mut s.config,
            Self::Class(c) => &mut c.config,
            Self::State(s) => &mut s.config,
            Self::Er(e) => &mut e.config,
            Self::Gantt(g) => &mut g.config,
        }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Flowchart(_) => "flowchart",
            Self::Sequence(_) => "sequence",
            Self::Class(_) => "class",
            Self::State(_) => "state",
            Self::Er(_) => "ER",
            Self::Gantt(_) => "gantt",
        }
    }
    pub fn summary(&self) -> String {
        match self {
            Self::Flowchart(g) => format!("{} nodes, {} edges", g.nodes.len(), g.edges.len()),
            Self::Gantt(g) => format!("{} tasks, {} sections", g.tasks.len(), g.sections.len()),
            Self::Er(e) => format!(
                "{} entities, {} relationships",
                e.entities.len(),
                e.relationships.len()
            ),
            Self::State(s) => format!(
                "{} states, {} transitions",
                s.states.len(),
                s.transitions.len()
            ),
            Self::Class(c) => format!(
                "{} classes, {} relationships",
                c.classes.len(),
                c.relations.len()
            ),
            Self::Sequence(s) => format!(
                "{} participants, {} events",
                s.participants.len(),
                s.events.len()
            ),
        }
    }
}
pub(crate) fn validate_config(c: &Config) -> Result<(), Diagnostic> {
    for v in [c.node_spacing, c.rank_spacing] {
        if !v.is_finite() || !(24.0..=300.0).contains(&v) {
            return Err(Diagnostic::new(
                "spacing must be between 24 and 300",
                Span {
                    line: 1,
                    column: 1,
                    ..Span::default()
                },
            ));
        }
    }
    Ok(())
}
