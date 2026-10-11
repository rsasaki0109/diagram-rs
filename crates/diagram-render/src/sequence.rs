use super::*;
use canvas::Canvas;
use diagram_core::sequence::*;
use diagram_layout::sequence::{EventGeometry, layout_sequence};
pub(super) fn render_sequence(seq: &Sequence, font: &Font<'_>) -> Result<String, Diagnostic> {
    let l = layout_sequence(seq, font)?;
    let mut c = Canvas::new(font, seq.config.theme);
    for (i, p) in seq.participants.iter().enumerate() {
        let r = l.participants[i];
        let cx = r.center().x;
        writeln!(
            c.body,
            "<g id=\"participant-{i}\" data-participant-id=\"{}\">",
            escape(&p.id)
        )
        .unwrap();
        for bottom in [false, true] {
            let y = if bottom {
                l.lifeline_bottom + 20.0
            } else {
                r.y
            };
            match p.kind {
                ParticipantKind::Participant => {
                    let label = font
                        .measure(&p.label, 15.0)
                        .map_err(|e| Diagnostic::new(e, p.span))?;
                    let rect = Rect {
                        x: r.x,
                        y: y + 24.0,
                        width: r.width,
                        height: label.height + 24.0,
                    };
                    c.rect(rect, c.fill, 4.0);
                    c.text(&p.label, rect.center(), 15.0)?;
                }
                ParticipantKind::Actor => {
                    writeln!(c.body,"<g fill=\"none\" stroke=\"{}\"><circle cx=\"{cx:.3}\" cy=\"{:.3}\" r=\"9\"/><path d=\"M {cx:.3} {:.3} v 22 m -18 -12 h 36 m -18 12 l -14 15 m 14 -15 l 14 15\"/></g>",c.stroke,y+10.0,y+19.0).unwrap();
                    let sz = font
                        .measure(&p.label, 15.0)
                        .map_err(|e| Diagnostic::new(e, p.span))?;
                    c.text(
                        &p.label,
                        Point {
                            x: cx,
                            y: y + 64.0 + sz.height / 2.0,
                        },
                        15.0,
                    )?;
                }
            }
        }
        writeln!(c.body,"<path fill=\"none\" stroke=\"{}\" stroke-dasharray=\"5 5\" d=\"M {cx:.3} {:.3} V {:.3}\"/></g>",c.stroke,r.y+r.height,l.lifeline_bottom).unwrap();
    }
    for (i, e) in l.events.iter().enumerate() {
        if let EventGeometry::Fragment { bounds, .. } = e {
            writeln!(c.body, "<g id=\"fragment-{i}\">").unwrap();
            c.rect(*bounds, "none", 0.0);
            c.body.push_str("</g>\n");
        }
    }
    for (i, a) in l.activations.iter().enumerate() {
        writeln!(
            c.body,
            "<g id=\"activation-{i}\" data-participant=\"{}\">",
            a.participant
        )
        .unwrap();
        c.rect(a.bounds, c.fill, 0.0);
        c.body.push_str("</g>\n");
    }
    let mut number = 0;
    for (i, (e, g)) in seq.events.iter().zip(&l.events).enumerate() {
        match (e, g) {
            (
                SequenceEvent::Message {
                    source,
                    target,
                    label,
                    kind,
                    ..
                },
                EventGeometry::Message {
                    points,
                    label: rect,
                },
            ) => {
                number += 1;
                let text = if seq.autonumber {
                    format!("{number}. {label}")
                } else {
                    label.clone()
                };
                let dotted = matches!(
                    kind,
                    MessageKind::DottedArrow | MessageKind::DottedOpen | MessageKind::DottedCross
                );
                let cross = matches!(kind, MessageKind::SolidCross | MessageKind::DottedCross);
                let open = matches!(kind, MessageKind::SolidOpen | MessageKind::DottedOpen);
                let path = if cross {
                    "M0 1 L8 9 M0 9 L8 1"
                } else if open {
                    "M0 1 L8 5 L0 9"
                } else {
                    "M0 1 L8 5 L0 9 Z"
                };
                writeln!(c.defs,"<marker id=\"arrow-{i}\" markerUnits=\"userSpaceOnUse\" markerWidth=\"10\" markerHeight=\"10\" refX=\"8\" refY=\"5\" orient=\"auto\"><path d=\"{path}\" stroke=\"{}\" fill=\"{}\"/></marker>",c.stroke,if cross||open {"none"} else {c.stroke}).unwrap();
                write!(c.body,"<path id=\"message-{i}\" data-source=\"{}\" data-target=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow-{i})\"",escape(&seq.participants[*source].id),escape(&seq.participants[*target].id),c.stroke).unwrap();
                if dotted {
                    c.body.push_str(" stroke-dasharray=\"5 4\"");
                }
                c.body.push_str(" d=\"");
                for (j, p) in points.iter().enumerate() {
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
                c.text(&text, rect.center(), 15.0)?;
            }
            (SequenceEvent::Note { label, .. }, EventGeometry::Note(r)) => {
                writeln!(c.body, "<g id=\"note-{i}\">").unwrap();
                c.rect(
                    *r,
                    if seq.config.theme == Theme::Dark {
                        "#374151"
                    } else {
                        "#fef3c7"
                    },
                    3.0,
                );
                c.text(label, r.center(), 15.0)?;
                c.body.push_str("</g>\n");
            }
            (
                SequenceEvent::FragmentStart { kind, label, .. },
                EventGeometry::Fragment { label: r, .. },
            ) => {
                c.text(&format!("{kind:?}: {label}"), r.center(), 15.0)?;
            }
            (SequenceEvent::Branch { label, .. }, EventGeometry::Branch { y, label: r }) => {
                writeln!(
                    c.body,
                    "<path stroke=\"{}\" stroke-dasharray=\"5 4\" d=\"M 20 {y:.3} H {:.3}\"/>",
                    c.stroke,
                    l.size.width - 20.0
                )
                .unwrap();
                c.text(label, r.center(), 15.0)?;
            }
            _ => {}
        }
    }
    Ok(c.finish(l.size, "Sequence diagram"))
}
