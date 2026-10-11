use super::*;
use canvas::Canvas;
use diagram_core::er::*;
use diagram_layout::layout_with_node_sizes;
fn keys(a: &Attribute) -> String {
    a.keys
        .iter()
        .map(|k| format!("{k:?}"))
        .collect::<Vec<_>>()
        .join(",")
}
fn marker(card: Cardinality, fill: &str, stroke: &str) -> String {
    let (head, tail) = match card {
        Cardinality::ExactlyOne => ("<path d=\"M9 1 V15 M15 1 V15\"/>", ""),
        Cardinality::ZeroOrOne => (
            "<circle cx=\"5\" cy=\"8\" r=\"4\"/>",
            "<path d=\"M15 1 V15\"/>",
        ),
        Cardinality::OneOrMore => (
            "<path d=\"M3 1 V15\"/>",
            "<path d=\"M6 8 L18 1 M6 8 L18 8 M6 8 L18 15\"/>",
        ),
        Cardinality::ZeroOrMore => (
            "<circle cx=\"4\" cy=\"8\" r=\"3\"/>",
            "<path d=\"M7 8 L18 1 M7 8 L18 8 M7 8 L18 15\"/>",
        ),
    };
    format!("<g fill=\"{fill}\" stroke=\"{stroke}\">{head}<g fill=\"none\">{tail}</g></g>")
}
pub(super) fn render_er(d: &ErDiagram, font: &Font<'_>) -> Result<String, Diagnostic> {
    d.validate()?;
    let mut g = Graph {
        direction: d.direction,
        config: d.config.clone(),
        ..Graph::default()
    };
    let mut sizes = Vec::new();
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let measure = |s: &str| {
        font.measure(s, 15.0).map_err(|e| {
            Diagnostic::new(
                e,
                Span {
                    line: 1,
                    column: 1,
                    ..Span::default()
                },
            )
        })
    };
    for e in &d.entities {
        let mut widths = [40.0_f64; 4];
        let mut heights = Vec::new();
        for a in &e.attributes {
            let mut height = 30.0_f64;
            for (i, text) in [
                a.data_type.clone(),
                a.name.clone(),
                keys(a),
                a.comment.clone(),
            ]
            .iter()
            .enumerate()
            {
                let sz = measure(text)?;
                widths[i] = widths[i].max(sz.width + 20.0);
                height = height.max(sz.height + 14.0);
            }
            heights.push(height);
        }
        if e.attributes.iter().all(|a| a.comment.is_empty()) {
            widths[3] = 0.0;
        }
        let title = measure(&e.id)?;
        let w = widths
            .iter()
            .sum::<f64>()
            .max(title.width + 32.0)
            .max(150.0);
        widths[1] += w - widths.iter().sum::<f64>();
        sizes.push(Size {
            width: w,
            height: title.height + 24.0 + heights.iter().sum::<f64>(),
        });
        columns.push(widths);
        rows.push(heights);
        g.nodes.push(Node {
            id: e.id.clone(),
            label: e.id.clone(),
            shape: Shape::Rectangle,
            style: Style::default(),
            group: None,
            span: e.span,
        });
    }
    for r in &d.relationships {
        g.edges.push(Edge {
            source: r.source,
            target: r.target,
            label: r.label.clone(),
            directed: false,
            source_arrow: false,
            min_rank_span: 1,
            kind: LineKind::Solid,
            style: Style::default(),
            span: r.span,
        });
    }
    let l = layout_with_node_sizes(&g, font, &sizes)?;
    let mut c = Canvas::new(font, d.config.theme);
    for (i, r) in d.relationships.iter().enumerate() {
        for (end, card) in [(0, r.source_cardinality), (1, r.target_cardinality)] {
            writeln!(c.defs,"<marker id=\"arrow-{i}-{end}\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"18\" refX=\"18\" refY=\"8\" orient=\"auto-start-reverse\">{}</marker>",marker(card,c.fill,c.stroke)).unwrap();
        }
        write!(c.body,"<path id=\"edge-{i}\" fill=\"none\" stroke=\"{}\" marker-start=\"url(#arrow-{i}-0)\" marker-end=\"url(#arrow-{i}-1)\"",c.stroke).unwrap();
        if !r.identifying {
            c.body.push_str(" stroke-dasharray=\"5 4\"");
        }
        c.body.push_str(" d=\"");
        for (j, p) in l.edges[i].points.iter().enumerate() {
            write!(
                c.body,
                "{} {:.3} {:.3} ",
                if j == 0 { "M" } else { "L" },
                p.x,
                p.y
            )
            .unwrap();
        }
        c.body.push_str("\"/>\n");
    }
    for (i, e) in d.entities.iter().enumerate() {
        let r = l.nodes[i];
        writeln!(
            c.body,
            "<g id=\"node-{i}\" data-entity-id=\"{}\">",
            escape(&e.id)
        )
        .unwrap();
        c.rect(r, c.fill, 0.0);
        let th = measure(&e.id)?.height + 24.0;
        c.text(
            &e.id,
            Point {
                x: r.center().x,
                y: r.y + th / 2.0,
            },
            15.0,
        )?;
        let mut y = r.y + th;
        for (j, a) in e.attributes.iter().enumerate() {
            writeln!(
                c.body,
                "<path stroke=\"{}\" d=\"M {:.3} {y:.3} H {:.3}\"/>",
                c.stroke,
                r.x,
                r.x + r.width
            )
            .unwrap();
            let mut x = r.x;
            for (col, text) in [
                a.data_type.clone(),
                a.name.clone(),
                keys(a),
                a.comment.clone(),
            ]
            .iter()
            .enumerate()
            {
                if columns[i][col] > 0.0 {
                    c.text(
                        text,
                        Point {
                            x: x + columns[i][col] / 2.0,
                            y: y + rows[i][j] / 2.0,
                        },
                        15.0,
                    )?;
                    x += columns[i][col];
                    if col < 3 && x < r.x + r.width - 2.0 {
                        writeln!(
                            c.body,
                            "<path stroke=\"{}\" d=\"M {x:.3} {y:.3} V {:.3}\"/>",
                            c.stroke,
                            y + rows[i][j]
                        )
                        .unwrap();
                    }
                }
            }
            y += rows[i][j];
        }
        c.body.push_str("</g>\n");
    }
    for (i, r) in d.relationships.iter().enumerate() {
        if !r.label.is_empty() {
            let rect = l.edges[i].label;
            c.rect(rect, c.background, 3.0);
            c.text(&r.label, rect.center(), 13.0)?;
        }
    }
    Ok(c.finish(l.size, "Entity relationship diagram"))
}
