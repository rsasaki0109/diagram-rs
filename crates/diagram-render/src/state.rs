use super::*;
use canvas::Canvas;
use diagram_core::state::*;
pub(super) fn render_state(d: &StateDiagram, font: &Font<'_>) -> Result<String, Diagnostic> {
    d.validate()?;
    let mut g = Graph {
        direction: d.direction,
        config: d.config.clone(),
        ..Graph::default()
    };
    for s in &d.states {
        let shape = match s.kind {
            StateKind::Regular => Shape::Rounded,
            StateKind::Initial => Shape::Initial,
            StateKind::Final => Shape::Final,
            StateKind::Choice => Shape::Diamond,
            StateKind::Fork | StateKind::Join => Shape::Bar,
        };
        let label = if s.description.is_empty() {
            s.label.clone()
        } else {
            format!("{}\n{}", s.label, s.description)
        };
        g.nodes.push(Node {
            id: s.id.clone(),
            label,
            shape,
            style: Style::default(),
            group: None,
            span: s.span,
        });
    }
    for t in &d.transitions {
        g.edges.push(Edge {
            source: t.source,
            target: t.target,
            label: t.label.clone(),
            directed: true,
            source_arrow: false,
            min_rank_span: 1,
            kind: LineKind::Solid,
            style: Style::default(),
            span: t.span,
        });
    }
    let l = layout(&g, font)?;
    let mut c = Canvas::new(font, d.config.theme);
    for (i, t) in d.transitions.iter().enumerate() {
        writeln!(c.defs,"<marker id=\"arrow-{i}\" markerUnits=\"userSpaceOnUse\" markerWidth=\"10\" markerHeight=\"10\" refX=\"10\" refY=\"5\" orient=\"auto\"><path d=\"M0 0 L10 5 L0 10 Z\" fill=\"{}\"/></marker>",c.stroke).unwrap();
        write!(c.body,"<path id=\"edge-{i}\" data-source=\"{}\" data-target=\"{}\" fill=\"none\" stroke=\"{}\" marker-end=\"url(#arrow-{i})\" d=\"",escape(&d.states[t.source].id),escape(&d.states[t.target].id),c.stroke).unwrap();
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
    for (i, s) in d.states.iter().enumerate() {
        let r = l.nodes[i];
        let center = r.center();
        writeln!(
            c.body,
            "<g id=\"node-{i}\" data-state-id=\"{}\">",
            escape(&s.id)
        )
        .unwrap();
        match s.kind {
            StateKind::Initial => {
                writeln!(
                    c.body,
                    "<circle cx=\"{:.3}\" cy=\"{:.3}\" r=\"{:.3}\" fill=\"{}\"/>",
                    center.x,
                    center.y,
                    r.width / 2.0,
                    c.ink
                )
                .unwrap();
            }
            StateKind::Final => {
                writeln!(c.body,"<circle cx=\"{:.3}\" cy=\"{:.3}\" r=\"{:.3}\" fill=\"{}\" stroke=\"{}\"/><circle cx=\"{:.3}\" cy=\"{:.3}\" r=\"{:.3}\" fill=\"{}\"/>",center.x,center.y,r.width/2.0,c.fill,c.ink,center.x,center.y,r.width/2.0-4.0,c.ink).unwrap();
            }
            StateKind::Fork | StateKind::Join => c.rect(r, c.ink, 0.0),
            StateKind::Choice => {
                writeln!(c.body,"<path fill=\"{}\" stroke=\"{}\" d=\"M {:.3} {:.3} L {:.3} {:.3} L {:.3} {:.3} L {:.3} {:.3} Z\"/>",c.fill,c.stroke,center.x,r.y,r.x+r.width,center.y,center.x,r.y+r.height,r.x,center.y).unwrap();
            }
            StateKind::Regular => {
                c.rect(r, c.fill, 8.0);
                c.text(&g.nodes[i].label, center, 15.0)?;
            }
        }
        c.body.push_str("</g>\n");
    }
    for (i, t) in d.transitions.iter().enumerate() {
        if !t.label.is_empty() {
            let r = l.edges[i].label;
            c.rect(r, c.background, 3.0);
            c.text(&t.label, r.center(), 13.0)?;
        }
    }
    Ok(c.finish(l.size, "State diagram"))
}
