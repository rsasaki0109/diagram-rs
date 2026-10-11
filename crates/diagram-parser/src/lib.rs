//! A native, source-located parser for a documented subset of Mermaid flowcharts.
use diagram_core::*;
use std::collections::BTreeMap;

/// Parse flowchart source. Unsupported syntax is an error, never silently discarded.
///
/// ```
/// let graph = diagram_parser::parse("flowchart TB; A[Start] --> B{Ready?}")?;
/// assert_eq!(graph.nodes.len(), 2);
/// assert_eq!(graph.edges.len(), 1);
/// # Ok::<(), diagram_core::Diagnostic>(())
/// ```
pub fn parse(source: &str) -> Result<Graph, Diagnostic> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(at(source, 0, "source exceeds 128 KiB"));
    }
    let (statements, config) = statements(source)?;
    let mut p = Parser {
        source,
        graph: Graph {
            config,
            ..Graph::default()
        },
        ids: BTreeMap::new(),
        classes: BTreeMap::new(),
        assignments: Vec::new(),
        node_styles: Vec::new(),
        edge_styles: Vec::new(),
        groups: Vec::new(),
    };
    let Some((head, off)) = statements.first() else {
        return Err(at(source, 0, "expected flowchart or graph header"));
    };
    let mut words = head.split_whitespace();
    if !matches!(words.next(), Some("flowchart" | "graph")) {
        return Err(at(
            source,
            *off,
            "unsupported diagram type; expected flowchart or graph",
        ));
    }
    p.graph.direction = match words.next() {
        Some("TB" | "TD") => Direction::TB,
        Some("BT") => Direction::BT,
        Some("LR") => Direction::LR,
        Some("RL") => Direction::RL,
        _ => return Err(at(source, *off, "expected direction TB, TD, BT, LR, or RL")),
    };
    if words.next().is_some() {
        return Err(at(
            source,
            *off,
            "unexpected content after header; separate statements with a newline or semicolon",
        ));
    }
    for (s, off) in statements.iter().skip(1) {
        p.statement(s, *off)?;
    }
    if !p.groups.is_empty() {
        return Err(at(source, source.len(), "unclosed subgraph; expected end"));
    }
    p.finish()?;
    p.graph.validate()?;
    Ok(p.graph)
}
fn span(source: &str, offset: usize, len: usize) -> Span {
    let prefix = &source[..offset.min(source.len())];
    Span {
        offset,
        len,
        line: prefix.bytes().filter(|&b| b == b'\n').count() + 1,
        column: prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1,
    }
}
fn at(source: &str, offset: usize, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, span(source, offset, 1))
}

fn statements(source: &str) -> Result<(Vec<(String, usize)>, Config), Diagnostic> {
    let mut out = Vec::new();
    let mut cfg = Config::default();
    let mut start = 0;
    let mut i = 0;
    let mut stack = Vec::new();
    let mut quote = false;
    let mut pipe = false;
    let b = source.as_bytes();
    while i < b.len() {
        if !quote && stack.is_empty() && !pipe && b[i..].starts_with(b"%%") {
            if source[start..i].trim().is_empty() && b[i..].starts_with(b"%%{") {
                let end = source[i + 3..]
                    .find("}%%")
                    .ok_or_else(|| at(source, i, "unclosed init directive"))?
                    + i
                    + 3;
                if !out.is_empty() {
                    return Err(at(source, i, "init directive must precede the header"));
                }
                let body = source[i + 3..end].trim();
                let json = body
                    .strip_prefix("init:")
                    .ok_or_else(|| at(source, i, "only a JSON init directive is supported"))?;
                cfg = parse_config(json).map_err(|e| at(source, i, e))?;
                i = end + 3;
                start = i;
                continue;
            }
            push_statement(source, start, i, &mut out);
            i = source[i..].find('\n').map_or(b.len(), |n| i + n);
            start = i;
            continue;
        }
        match b[i] {
            b'"' if !escaped(source, i) => quote = !quote,
            b'|' if !quote && stack.is_empty() => pipe = !pipe,
            b'[' | b'(' | b'{' if !quote && !pipe => stack.push(b[i]),
            b']' | b')' | b'}' if !quote && !pipe => {
                let expected = match b[i] {
                    b']' => b'[',
                    b')' => b'(',
                    _ => b'{',
                };
                if stack.pop() != Some(expected) {
                    return Err(at(source, i, "unmatched label delimiter"));
                }
            }
            b';' | b'\n' if !quote && !pipe && stack.is_empty() => {
                push_statement(source, start, i, &mut out);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if quote || pipe || !stack.is_empty() {
        return Err(at(source, start, "unclosed label, quote, or edge label"));
    }
    push_statement(source, start, b.len(), &mut out);
    Ok((out, cfg))
}
fn push_statement(source: &str, start: usize, end: usize, out: &mut Vec<(String, usize)>) {
    let raw = &source[start..end];
    let s = raw.trim();
    if !s.is_empty() {
        out.push((s.to_owned(), start + raw.len() - raw.trim_start().len()));
    }
}
fn parse_config(s: &str) -> Result<Config, String> {
    let v: serde_json::Value =
        serde_json::from_str(s).map_err(|e| format!("init requires strict JSON: {e}"))?;
    let obj = v
        .as_object()
        .ok_or("init configuration must be an object")?;
    let mut c = Config::default();
    for (key, val) in obj {
        match key.as_str() {
            "theme" => {
                c.theme = match val.as_str() {
                    Some("dark") => Theme::Dark,
                    Some("default" | "light") => Theme::Light,
                    _ => return Err("supported themes: default, light, dark".into()),
                }
            }
            "flowchart" => {
                for (key, val) in val
                    .as_object()
                    .ok_or("flowchart config must be an object")?
                {
                    let n = val
                        .as_f64()
                        .filter(|n| n.is_finite() && (24.0..=300.0).contains(n))
                        .ok_or("spacing must be a number between 24 and 300")?;
                    match key.as_str() {
                        "nodeSpacing" => c.node_spacing = n,
                        "rankSpacing" => c.rank_spacing = n,
                        _ => return Err(format!("unsupported flowchart configuration: {key}")),
                    }
                }
            }
            _ => return Err(format!("unsupported configuration: {key}")),
        }
    }
    Ok(c)
}
struct Parser<'a> {
    source: &'a str,
    graph: Graph,
    ids: BTreeMap<String, usize>,
    classes: BTreeMap<String, Style>,
    assignments: Vec<(Vec<String>, Vec<String>, Span)>,
    node_styles: Vec<(Vec<String>, Style, Span)>,
    edge_styles: Vec<(Option<Vec<usize>>, Style, Span)>,
    groups: Vec<usize>,
}
impl Parser<'_> {
    fn statement(&mut self, s: &str, off: usize) -> Result<(), Diagnostic> {
        let loc = span(self.source, off, s.len());
        if s == "end" {
            if self.groups.pop().is_none() {
                return Err(at(self.source, off, "unexpected end"));
            }
            return Ok(());
        }
        if let Some(rest) = keyword(s, "subgraph") {
            if self.groups.len() >= 32 || self.graph.groups.len() >= MAX_NODES {
                return Err(at(
                    self.source,
                    off,
                    "subgraph depth exceeds 32 or count exceeds 512",
                ));
            }
            let mut cur = Cursor::new(rest, self.source, off + s.len() - rest.len());
            let id = cur.id()?;
            cur.ws();
            let label = if cur.peek() == Some(b'[') {
                cur.label(b'[', b']', false)?
            } else if cur.done() {
                id.clone()
            } else {
                return Err(cur.error("use subgraph id [Title] for titles containing spaces"));
            };
            cur.end()?;
            let parent = self.groups.last().copied();
            self.groups.push(self.graph.groups.len());
            self.graph.groups.push(Group {
                parent,
                id,
                label,
                span: loc,
            });
            return Ok(());
        }
        if let Some(rest) = keyword(s, "classDef") {
            let (names, styles) = split_arg(rest)
                .ok_or_else(|| at(self.source, off, "classDef requires names and properties"))?;
            let style = parse_style(styles).map_err(|e| at(self.source, off, e))?;
            for name in names.split(',') {
                validate_id(name).map_err(|e| at(self.source, off, e))?;
                self.classes.entry(name.into()).or_default().overlay(&style);
            }
            return Ok(());
        }
        if let Some(rest) = keyword(s, "class") {
            let (ids, names) = split_arg(rest)
                .ok_or_else(|| at(self.source, off, "class requires node ids and class names"))?;
            let ids = parse_ids(ids).map_err(|e| at(self.source, off, e))?;
            let names = parse_ids(names).map_err(|e| at(self.source, off, e))?;
            self.assignments.push((ids, names, loc));
            return Ok(());
        }
        if let Some(rest) = keyword(s, "style") {
            let (ids, styles) = split_arg(rest)
                .ok_or_else(|| at(self.source, off, "style requires node ids and properties"))?;
            self.node_styles.push((
                parse_ids(ids).map_err(|e| at(self.source, off, e))?,
                parse_style(styles).map_err(|e| at(self.source, off, e))?,
                loc,
            ));
            return Ok(());
        }
        if let Some(rest) = keyword(s, "linkStyle") {
            let (indices, styles) = split_arg(rest).ok_or_else(|| {
                at(
                    self.source,
                    off,
                    "linkStyle requires edge indices and properties",
                )
            })?;
            let ids = if indices == "default" {
                None
            } else {
                Some(
                    indices
                        .split(',')
                        .map(|v| {
                            v.parse::<usize>()
                                .map_err(|_| at(self.source, off, "invalid edge index"))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                )
            };
            self.edge_styles.push((
                ids,
                parse_style(styles).map_err(|e| at(self.source, off, e))?,
                loc,
            ));
            return Ok(());
        }
        if matches!(
            s.split_whitespace().next(),
            Some("click" | "direction" | "accTitle:" | "accDescr:")
        ) {
            return Err(at(self.source, off, "unsupported flowchart statement"));
        }
        let mut cur = Cursor::new(s, self.source, off);
        let mut left = self.node_set(&mut cur)?;
        while !cur.done() {
            let edge = cur.edge()?;
            let right = self.node_set(&mut cur)?;
            for &a in &left {
                for &b in &right {
                    if self.graph.edges.len() >= MAX_EDGES {
                        return Err(cur.error("diagram exceeds 2048 edges"));
                    }
                    self.graph.edges.push(Edge {
                        source: a,
                        target: b,
                        directed: edge.directed,
                        source_arrow: edge.source_arrow,
                        min_rank_span: edge.min_rank_span,
                        kind: edge.kind,
                        label: edge.label.clone(),
                        style: Style::default(),
                        span: loc,
                    });
                }
            }
            left = right;
        }
        Ok(())
    }
    fn node_set(&mut self, cur: &mut Cursor<'_>) -> Result<Vec<usize>, Diagnostic> {
        let mut nodes = vec![self.node(cur)?];
        loop {
            cur.ws();
            if cur.peek() != Some(b'&') {
                break;
            }
            cur.i += 1;
            nodes.push(self.node(cur)?);
        }
        Ok(nodes)
    }
    fn node(&mut self, cur: &mut Cursor<'_>) -> Result<usize, Diagnostic> {
        cur.ws();
        let start = cur.i;
        let id = cur.id()?;
        cur.ws();
        let shape = match cur.peek() {
            Some(b'[') if cur.s[cur.i..].starts_with("[/") || cur.s[cur.i..].starts_with("[\\") => {
                return Err(cur.error("parallelogram and trapezoid shapes are not supported"));
            }
            Some(b'[') if cur.s[cur.i..].starts_with("[[") => {
                Some((Shape::Subroutine, cur.label_pair("[[", "]]")?))
            }
            Some(b'[') if cur.s[cur.i..].starts_with("[(") => {
                Some((Shape::Cylinder, cur.label_pair("[(", ")]")?))
            }
            Some(b'[') => Some((Shape::Rectangle, cur.label(b'[', b']', false)?)),
            Some(b'{') => Some((Shape::Diamond, cur.label(b'{', b'}', false)?)),
            Some(b'(') => {
                if cur.s[cur.i..].starts_with("([") {
                    Some((Shape::Stadium, cur.label_pair("([", "])")?))
                } else if cur.s[cur.i..].starts_with("((") {
                    Some((Shape::Circle, cur.label(b'(', b')', true)?))
                } else {
                    Some((Shape::Rounded, cur.label(b'(', b')', false)?))
                }
            }
            _ => None,
        };
        let n = if let Some(&n) = self.ids.get(&id) {
            if let Some((shape, label)) = shape {
                self.graph.nodes[n].shape = shape;
                self.graph.nodes[n].label = label;
            }
            if let Some(group) = self.groups.last().copied() {
                if self.graph.nodes[n].group.is_some_and(|g| {
                    g != group
                        && !self.graph.group_contains(Some(g), group)
                        && !self.graph.group_contains(Some(group), g)
                }) {
                    return Err(cur.error("node cannot belong to multiple subgraphs"));
                }
                if !self.graph.group_contains(self.graph.nodes[n].group, group) {
                    self.graph.nodes[n].group = Some(group);
                }
            }
            n
        } else {
            if self.graph.nodes.len() >= MAX_NODES {
                return Err(cur.error("diagram exceeds 512 nodes"));
            }
            let (shape, label) = shape.unwrap_or((Shape::Rectangle, id.clone()));
            let n = self.graph.nodes.len();
            self.ids.insert(id.clone(), n);
            self.graph.nodes.push(Node {
                id: id.clone(),
                label,
                shape,
                style: Style::default(),
                group: self.groups.last().copied(),
                span: span(self.source, cur.off + start, cur.i - start),
            });
            n
        };
        cur.ws();
        if cur.s[cur.i..].starts_with(":::") {
            cur.i += 3;
            let name = cur.id()?;
            self.assignments.push((
                vec![id],
                vec![name],
                span(self.source, cur.off + start, cur.i - start),
            ));
        }
        Ok(n)
    }
    fn finish(&mut self) -> Result<(), Diagnostic> {
        if let Some(style) = self.classes.get("default") {
            for n in &mut self.graph.nodes {
                n.style.overlay(style);
            }
        }
        let mut explicitly_classed = std::collections::BTreeSet::new();
        for (ids, names, loc) in &self.assignments {
            for id in ids {
                let &n = self
                    .ids
                    .get(id)
                    .ok_or_else(|| Diagnostic::new(format!("unknown node: {id}"), *loc))?;
                // Explicit classes replace default class styling.
                if explicitly_classed.insert(n) {
                    self.graph.nodes[n].style = Style::default();
                }
                for name in names {
                    let style = self
                        .classes
                        .get(name)
                        .ok_or_else(|| Diagnostic::new(format!("undefined class: {name}"), *loc))?;
                    self.graph.nodes[n].style.overlay(style);
                }
            }
        }
        for (ids, style, loc) in &self.node_styles {
            for id in ids {
                let &n = self
                    .ids
                    .get(id)
                    .ok_or_else(|| Diagnostic::new(format!("unknown node: {id}"), *loc))?;
                self.graph.nodes[n].style.overlay(style);
            }
        }
        for (indices, style, loc) in &self.edge_styles {
            match indices {
                None => {
                    for e in &mut self.graph.edges {
                        e.style.overlay(style);
                    }
                }
                Some(indices) => {
                    for &i in indices {
                        self.graph
                            .edges
                            .get_mut(i)
                            .ok_or_else(|| {
                                Diagnostic::new(format!("edge index {i} out of range"), *loc)
                            })?
                            .style
                            .overlay(style);
                    }
                }
            }
        }
        Ok(())
    }
}
fn keyword<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    s.strip_prefix(key)
        .filter(|rest| rest.starts_with(char::is_whitespace))
        .map(str::trim_start)
}
fn split_arg(s: &str) -> Option<(&str, &str)> {
    let i = s.find(char::is_whitespace)?;
    Some((&s[..i], s[i..].trim()))
}
fn validate_id(s: &str) -> Result<(), String> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        Err("identifiers must contain ASCII letters, digits, or underscores".into())
    } else {
        Ok(())
    }
}
fn parse_ids(s: &str) -> Result<Vec<String>, String> {
    s.split(',')
        .map(|s| {
            validate_id(s)?;
            Ok(s.into())
        })
        .collect()
}
fn parse_style(s: &str) -> Result<Style, String> {
    let mut style = Style::default();
    for entry in s.split(',') {
        let (key, value) = entry
            .split_once(':')
            .ok_or("expected style property:value")?;
        let value = value.trim();
        match key.trim() {
            "fill" => style.fill = Some(value.into()),
            "stroke" => style.stroke = Some(value.into()),
            "color" => style.color = Some(value.into()),
            "stroke-width" | "font-size" => {
                let n = value
                    .strip_suffix("px")
                    .unwrap_or(value)
                    .parse::<f64>()
                    .map_err(|_| "expected numeric style value")?;
                if key.trim() == "stroke-width" {
                    style.stroke_width = Some(n);
                } else {
                    style.font_size = Some(n);
                }
            }
            _ => return Err(format!("unsupported style property: {}", key.trim())),
        }
    }
    style.validate()?;
    Ok(style)
}
struct EdgeSpec {
    directed: bool,
    source_arrow: bool,
    min_rank_span: usize,
    kind: LineKind,
    label: String,
}
struct Cursor<'a> {
    s: &'a str,
    source: &'a str,
    off: usize,
    i: usize,
}
impl<'a> Cursor<'a> {
    fn new(s: &'a str, source: &'a str, off: usize) -> Self {
        Self {
            s,
            source,
            off,
            i: 0,
        }
    }
    fn peek(&self) -> Option<u8> {
        self.s.as_bytes().get(self.i).copied()
    }
    fn ws(&mut self) {
        while self.peek().is_some_and(|b| b.is_ascii_whitespace()) {
            self.i += 1;
        }
    }
    fn done(&mut self) -> bool {
        self.ws();
        self.i == self.s.len()
    }
    fn error(&self, msg: impl Into<String>) -> Diagnostic {
        at(self.source, self.off + self.i, msg)
    }
    fn end(&mut self) -> Result<(), Diagnostic> {
        if self.done() {
            Ok(())
        } else {
            Err(self.error("unexpected or unsupported syntax"))
        }
    }
    fn id(&mut self) -> Result<String, Diagnostic> {
        self.ws();
        let start = self.i;
        while self
            .peek()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            self.i += 1;
        }
        if start == self.i {
            return Err(self.error("expected node identifier (ASCII letters, digits, underscores)"));
        }
        Ok(self.s[start..self.i].into())
    }
    fn label(&mut self, open: u8, close: u8, double: bool) -> Result<String, Diagnostic> {
        let opens = String::from_utf8(vec![open; if double { 2 } else { 1 }]).unwrap();
        let closes = String::from_utf8(vec![close; if double { 2 } else { 1 }]).unwrap();
        self.label_pair(&opens, &closes)
    }
    fn label_pair(&mut self, open: &str, close: &str) -> Result<String, Diagnostic> {
        self.i += open.len();
        let start = self.i;
        let mut quote = false;
        while let Some(b) = self.peek() {
            if b == b'"' && !escaped(self.s, self.i) {
                quote = !quote;
            }
            // Comparing bytes keeps this scan safe inside a UTF-8 character.
            if !quote && self.s.as_bytes()[self.i..].starts_with(close.as_bytes()) {
                let raw = &self.s[start..self.i];
                self.i += close.len();
                return decode_label(raw.trim()).map_err(|e| self.error(e));
            }
            if !quote && matches!(b, b'[' | b'{' | b'(' | b']' | b'}' | b')') {
                return Err(self.error("unsupported node shape or unquoted delimiter in label"));
            }
            self.i += 1;
        }
        Err(self.error("unclosed node label"))
    }
    fn connector(&mut self) -> Result<EdgeSpec, Diagnostic> {
        let source_arrow = self.peek() == Some(b'<');
        if source_arrow {
            self.i += 1;
        }
        let token = self
            .peek()
            .ok_or_else(|| self.error("expected connector"))?;
        let kind = match token {
            b'-' => LineKind::Solid,
            b'=' => LineKind::Thick,
            _ => return Err(self.error("unsupported connector")),
        };
        let start = self.i;
        while self.peek() == Some(token) {
            self.i += 1;
        }
        let count = self.i - start;
        let (kind, count, dotted) = if token == b'-' && count == 1 && self.peek() == Some(b'.') {
            let start = self.i;
            while self.peek() == Some(b'.') {
                self.i += 1;
            }
            let count = self.i - start;
            if self.peek() != Some(b'-') {
                return Err(self.error("dotted connector must end with - or ->"));
            }
            self.i += 1;
            (LineKind::Dotted, count, true)
        } else {
            (kind, count, false)
        };
        let directed = self.peek() == Some(b'>');
        if directed {
            self.i += 1;
        }
        let base = if dotted {
            0
        } else if directed {
            1
        } else {
            2
        };
        if count <= base || source_arrow && !directed {
            return Err(self.error("unsupported connector; use -->, ---, -.->, -.-, ==>, ===, or a bidirectional variant"));
        }
        // Mermaid's flowchart database clamps encoded link lengths to ten.
        // The byte limit already bounds the scan, including very long tokens.
        let min_rank_span = (count - base).min(10);
        Ok(EdgeSpec {
            directed,
            source_arrow,
            min_rank_span,
            kind,
            label: String::new(),
        })
    }
    fn edge(&mut self) -> Result<EdgeSpec, Diagnostic> {
        self.ws();
        let mut edge = if self.s[self.i..].starts_with("-- ") {
            self.i += 2;
            let start = self.i;
            let mut quote = false;
            while let Some(b) = self.peek() {
                if b == b'"' && !escaped(self.s, self.i) {
                    quote = !quote;
                }
                if b == b'-' && !quote && self.s.as_bytes()[self.i..].starts_with(b"--") {
                    break;
                }
                self.i += 1;
            }
            let end = self.i;
            let label = decode_label(self.s[start..end].trim()).map_err(|e| self.error(e))?;
            let mut edge = self.connector()?;
            if !edge.directed || edge.kind != LineKind::Solid {
                return Err(self.error("expected --> after edge label"));
            }
            edge.label = label;
            edge
        } else {
            self.connector()?
        };
        self.ws();
        if self.peek() == Some(b'|') {
            if !edge.label.is_empty() {
                return Err(self.error("edge has two labels"));
            }
            self.i += 1;
            let start = self.i;
            let end = self.s[start..]
                .find('|')
                .ok_or_else(|| self.error("unclosed edge label"))?
                + start;
            edge.label = decode_label(self.s[start..end].trim()).map_err(|e| self.error(e))?;
            self.i = end + 1;
        }
        Ok(edge)
    }
}
fn decode_label(raw: &str) -> Result<String, String> {
    let raw = if raw.starts_with('"') {
        raw.strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .ok_or("unclosed quoted label")?
    } else {
        raw
    };
    let mut s = raw
        .replace("\r\n", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n")
        .replace("\\n", "\n")
        .replace("\\\"", "\"");
    for (from, to) in [
        ("#quot;", "\""),
        ("#amp;", "&"),
        ("#lt;", "<"),
        ("#gt;", ">"),
        ("&quot;", "\""),
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
    ] {
        s = s.replace(from, to);
    }
    // Mermaid's decimal entity syntax (#9829;) is distinct from XML (&#9829;).
    let mut out = String::new();
    let mut rest = s.as_str();
    while let Some(i) = rest.find('#') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        if let Some(end) = rest.find(';').filter(|&e| e < 10)
            && let Ok(n) = rest[1..end].parse::<u32>()
        {
            let c = char::from_u32(n).ok_or("invalid Unicode entity")?;
            out.push(c);
            rest = &rest[end + 1..];
            continue;
        }
        out.push('#');
        rest = &rest[1..];
    }
    out.push_str(rest);
    Ok(out)
}

fn escaped(source: &str, offset: usize) -> bool {
    source.as_bytes()[..offset]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count()
        % 2
        == 1
}

mod class;
mod er;
mod gantt;
mod sequence;
mod state;
/// Parse any implemented diagram family. The flowchart-only `parse` API is retained.
pub fn parse_diagram(source: &str) -> Result<Diagram, Diagnostic> {
    let (lines, _) = family_lines(source)?;
    let Some((header, off)) = lines.first() else {
        return Err(at(source, 0, "expected diagram header"));
    };
    match header.split_whitespace().next() {
        Some("flowchart" | "graph") => parse(source).map(Diagram::Flowchart),
        Some("sequenceDiagram") if header == "sequenceDiagram" => {
            sequence::parse_sequence(source).map(Diagram::Sequence)
        }
        Some("classDiagram") if header == "classDiagram" => {
            class::parse_class(source).map(Diagram::Class)
        }
        Some("stateDiagram" | "stateDiagram-v2")
            if matches!(header.as_str(), "stateDiagram" | "stateDiagram-v2") =>
        {
            state::parse_state(source).map(Diagram::State)
        }
        Some("erDiagram") if header == "erDiagram" => er::parse_er(source).map(Diagram::Er),
        Some("gantt") if header == "gantt" => gantt::parse_gantt(source).map(Diagram::Gantt),
        _ => Err(at(source, *off, "unsupported diagram header")),
    }
}
/// Line/semicolon scanner for diagram families whose labels do not use bracket shapes.
fn family_lines(source: &str) -> Result<(Vec<(String, usize)>, Config), Diagnostic> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(at(source, 0, "source exceeds 128 KiB"));
    }
    let mut lines = Vec::new();
    let mut config = Config::default();
    let mut i = 0;
    let mut start = 0;
    let mut quoted = false;
    let b = source.as_bytes();
    while i < b.len() {
        if !quoted && b[i..].starts_with(b"%%") {
            if source[start..i].trim().is_empty() && b[i..].starts_with(b"%%{") {
                let end = source[i + 3..]
                    .find("}%%")
                    .ok_or_else(|| at(source, i, "unclosed init directive"))?
                    + i
                    + 3;
                let body = source[i + 3..end]
                    .trim()
                    .strip_prefix("init:")
                    .ok_or_else(|| at(source, i, "only init directives are supported"))?;
                config = parse_config(body).map_err(|e| at(source, i, e))?;
                i = end + 3;
                start = i;
                continue;
            }
            push_statement(source, start, i, &mut lines);
            i = source[i..].find('\n').map_or(b.len(), |n| i + n);
            start = i;
            continue;
        }
        if b[i] == b'"' && !escaped(source, i) {
            quoted = !quoted;
        }
        if !quoted && matches!(b[i], b'\n' | b';') {
            push_statement(source, start, i, &mut lines);
            start = i + 1;
        }
        i += 1;
    }
    if quoted {
        return Err(at(source, start, "unclosed quoted label"));
    }
    push_statement(source, start, b.len(), &mut lines);
    Ok((lines, config))
}
fn direction(s: &str) -> Result<Direction, String> {
    match s {
        "TB" | "TD" => Ok(Direction::TB),
        "BT" => Ok(Direction::BT),
        "LR" => Ok(Direction::LR),
        "RL" => Ok(Direction::RL),
        _ => Err("expected direction TB, TD, BT, LR or RL".into()),
    }
}
