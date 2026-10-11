use super::*;
use canvas::Canvas;
use diagram_core::class::*;
fn compartment_title(c: &Class) -> String {
    match &c.annotation {
        Some(a) => format!("«{a}»\n{}", c.label),
        None => c.label.clone(),
    }
}
fn compartment(lines: &[String]) -> String {
    if lines.is_empty() {
        " ".into()
    } else {
        lines.join("\n")
    }
}
pub(super) fn render_class(d: &ClassDiagram, font: &Font<'_>) -> Result<String, Diagnostic> {
    d.validate()?;
    let mut g = Graph {
        direction: d.direction,
        config: d.config.clone(),
        ..Graph::default()
    };
    for c in &d.classes {
        g.nodes.push(Node {
            id: c.id.clone(),
            label: format!(
                "{}\n{}\n{}",
                compartment_title(c),
                compartment(&c.attributes),
                compartment(&c.methods)
            ),
            shape: Shape::Rectangle,
            style: Style::default(),
            group: None,
            span: c.span,
        });
    }
    for r in &d.relations {
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
    let l = layout(&g, font)?;
    let mut c = Canvas::new(font, d.config.theme);
    for (i, r) in d.relations.iter().enumerate() {
        let dashed = r.dashed;
        let (shape, fill) = match r.kind {
            ClassRelationKind::Inheritance | ClassRelationKind::Realization => {
                ("M0 0 L14 7 L0 14 Z", c.fill)
            }
            ClassRelationKind::Composition => ("M0 7 L7 0 L14 7 L7 14 Z", c.stroke),
            ClassRelationKind::Aggregation => ("M0 7 L7 0 L14 7 L7 14 Z", c.fill),
            ClassRelationKind::Association | ClassRelationKind::Dependency => {
                ("M0 1 L14 7 L0 13", "none")
            }
            ClassRelationKind::Link => ("", "none"),
        };
        if !shape.is_empty() {
            writeln!(c.defs,"<marker id=\"arrow-{i}\" markerUnits=\"userSpaceOnUse\" markerWidth=\"16\" markerHeight=\"16\" refX=\"14\" refY=\"7\" orient=\"auto-start-reverse\"><path d=\"{shape}\" fill=\"{fill}\" stroke=\"{}\"/></marker>",c.stroke).unwrap();
        }
        write!(
            c.body,
            "<path id=\"edge-{i}\" fill=\"none\" stroke=\"{}\"",
            c.stroke
        )
        .unwrap();
        if dashed {
            c.body.push_str(" stroke-dasharray=\"5 4\"");
        }
        if !shape.is_empty() {
            write!(
                c.body,
                " marker-{}=\"url(#arrow-{i})\"",
                if r.reversed { "start" } else { "end" }
            )
            .unwrap();
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
    for (i, cl) in d.classes.iter().enumerate() {
        let r = l.nodes[i];
        writeln!(
            c.body,
            "<g id=\"node-{i}\" data-class-id=\"{}\">",
            escape(&cl.id)
        )
        .unwrap();
        c.rect(r, c.fill, 0.0);
        let mut y = r.y;
        for (j, text) in [
            compartment_title(cl),
            compartment(&cl.attributes),
            compartment(&cl.methods),
        ]
        .iter()
        .enumerate()
        {
            let sz = font
                .measure(text, 15.0)
                .map_err(|e| Diagnostic::new(e, cl.span))?;
            c.text(
                text,
                Point {
                    x: r.center().x,
                    y: y + 4.0 + sz.height / 2.0,
                },
                15.0,
            )?;
            y += sz.height + 8.0;
            if j < 2 {
                writeln!(
                    c.body,
                    "<path stroke=\"{}\" d=\"M {:.3} {y:.3} H {:.3}\"/>",
                    c.stroke,
                    r.x,
                    r.x + r.width
                )
                .unwrap();
            }
        }
        c.body.push_str("</g>\n");
    }
    for (i, r) in d.relations.iter().enumerate() {
        if !r.label.is_empty() {
            let rect = l.edges[i].label;
            c.rect(rect, c.background, 3.0);
            c.text(&r.label, rect.center(), 13.0)?;
        }
        let ps = &l.edges[i].points;
        for (label, p, q) in [
            (&r.source_cardinality, ps[0], ps[1]),
            (&r.target_cardinality, *ps.last().unwrap(), ps[ps.len() - 2]),
        ] {
            if !label.is_empty() {
                let dx = (q.x - p.x).signum();
                let dy = (q.y - p.y).signum();
                c.text(
                    label,
                    Point {
                        x: p.x + dx * 22.0 + dy * 12.0,
                        y: p.y + dy * 22.0 - dx * 12.0,
                    },
                    12.0,
                )?;
            }
        }
    }
    Ok(c.finish(l.size, "Class diagram"))
}
