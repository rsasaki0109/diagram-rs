use super::*;
use canvas::Canvas;
use diagram_core::gantt::*;
use diagram_layout::gantt::layout_gantt;
pub(super) fn render_gantt(g: &Gantt, font: &Font<'_>) -> Result<String, Diagnostic> {
    let l = layout_gantt(g, font)?;
    let mut c = Canvas::new(font, g.config.theme);
    c.text(&g.title, l.title.center(), 15.0)?;
    for (date, x) in &l.ticks {
        writeln!(
            c.body,
            "<path stroke=\"{}\" stroke-opacity=\"0.25\" d=\"M {x:.3} {:.3} V {:.3}\"/>",
            c.stroke,
            l.axis_y,
            l.size.height - 20.0
        )
        .unwrap();
        c.text(
            &date.iso(),
            Point {
                x: *x,
                y: l.axis_y - 20.0,
            },
            12.0,
        )?;
    }
    for s in &l.sections {
        let band = Rect {
            x: 10.0,
            y: s.label.y - 5.0,
            width: l.size.width - 20.0,
            height: s.label.height + 10.0,
        };
        c.rect(band, c.fill, 0.0);
        c.text(&g.sections[s.section], s.label.center(), 15.0)?;
    }
    for (i, t) in g.tasks.iter().enumerate() {
        for (j, &dep) in t.dependencies.iter().enumerate() {
            let source = l.tasks[dep].bar;
            let target = l.tasks[i].bar;
            let sx = source.x + source.width;
            let sy = source.center().y;
            let tx = target.x;
            let ty = target.center().y;
            writeln!(c.defs,"<marker id=\"arrow-{i}-{j}\" markerUnits=\"userSpaceOnUse\" markerWidth=\"8\" markerHeight=\"8\" refX=\"7\" refY=\"4\" orient=\"auto\"><path d=\"M0 0 L7 4 L0 8 Z\" fill=\"{}\"/></marker>",c.stroke).unwrap();
            let lane = (sx + tx) / 2.0;
            writeln!(c.body,"<path id=\"edge-{i}-{j}\" fill=\"none\" stroke=\"{}\" stroke-opacity=\"0.65\" marker-end=\"url(#arrow-{i}-{j})\" d=\"M {sx:.3} {sy:.3} H {lane:.3} V {ty:.3} H {tx:.3}\"/>",c.stroke).unwrap();
        }
    }
    for (i, (t, r)) in g.tasks.iter().zip(&l.tasks).enumerate() {
        writeln!(c.body,"<g id=\"task-{i}\" data-task-id=\"{}\" data-start=\"{}\" data-end=\"{}\"><title>{}: {} to {}</title>",escape(&t.id),t.start.iso(),t.end.iso(),escape(&t.label),t.start.iso(),t.end.iso()).unwrap();
        let color = if t.critical {
            "#dc2626"
        } else {
            match t.status {
                TaskStatus::Normal => "#93c5fd",
                TaskStatus::Active => "#2563eb",
                TaskStatus::Done => "#94a3b8",
            }
        };
        if t.milestone {
            let center = Point {
                x: r.bar.x,
                y: r.bar.center().y,
            };
            writeln!(c.body,"<path fill=\"{color}\" stroke=\"{}\" d=\"M {:.3} {:.3} l 10 12 l -10 12 l -10 -12 Z\"/>",c.stroke,center.x,center.y-12.0).unwrap();
        } else {
            c.rect(r.bar, color, 3.0);
        }
        c.text(&t.label, r.label.center(), 15.0)?;
        c.body.push_str("</g>\n");
    }
    Ok(c.finish(l.size, "Gantt diagram"))
}
