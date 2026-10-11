//! Measured lifeline/timeline layout for sequence diagrams.
use diagram_core::{sequence::*, *};

#[derive(Clone, Debug, PartialEq)]
pub enum EventGeometry {
    Message { points: Vec<Point>, label: Rect },
    Note(Rect),
    Fragment { bounds: Rect, label: Rect },
    Branch { y: f64, label: Rect },
    Empty,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Activation {
    pub participant: usize,
    pub bounds: Rect,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SequenceLayout {
    pub participants: Vec<Rect>,
    pub events: Vec<EventGeometry>,
    pub activations: Vec<Activation>,
    pub size: Size,
    pub lifeline_bottom: f64,
}
pub fn layout_sequence(
    seq: &Sequence,
    metrics: &impl TextMeasure,
) -> Result<SequenceLayout, Diagnostic> {
    seq.validate()?;
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
    let measure = |s: &str| metrics.measure(s, 15.0).map_err(fail);
    let n = seq.participants.len();
    let mut widths = Vec::new();
    let mut header_height = 80.0_f64;
    for p in &seq.participants {
        let s = measure(&p.label)?;
        widths.push((s.width + 32.0).max(110.0));
        header_height = header_height.max(s.height + 64.0);
    }
    let mut gaps = vec![seq.config.node_spacing.max(40.0); n.saturating_sub(1)];
    let mut left = 40.0_f64;
    let mut right = 40.0_f64;
    let mut depth = 0_usize;
    let mut max_depth = 0_usize;
    let mut frame_width = 0.0_f64;
    for e in &seq.events {
        match e {
            SequenceEvent::Message {
                source,
                target,
                label,
                ..
            } => {
                let sz = measure(label)?;
                if source == target {
                    if *source + 1 < n {
                        gaps[*source] = gaps[*source].max(sz.width + 190.0);
                    } else {
                        right = right.max(sz.width + 190.0);
                    }
                } else {
                    let (a, b) = ((*source).min(*target), (*source).max(*target));
                    for gap in &mut gaps[a..b] {
                        *gap = gap.max(sz.width / (b - a) as f64 + 30.0);
                    }
                }
            }
            SequenceEvent::Note {
                participants,
                position,
                label,
                ..
            } => {
                let width = measure(label)?.width + 48.0;
                let p = participants[0];
                match position {
                    NotePosition::Over if participants.len() == 1 => {
                        widths[p] = widths[p].max(width)
                    }
                    NotePosition::Over if participants.len() == 2 => {
                        let (a, b) = (
                            participants[0].min(participants[1]),
                            participants[0].max(participants[1]),
                        );
                        if a == b {
                            widths[a] = widths[a].max(width);
                        } else {
                            for gap in &mut gaps[a..b] {
                                *gap = gap.max(width / (b - a) as f64);
                            }
                        }
                    }
                    NotePosition::LeftOf if p == 0 => left = left.max(width),
                    NotePosition::RightOf if p + 1 == n => right = right.max(width),
                    NotePosition::LeftOf => gaps[p - 1] = gaps[p - 1].max(width),
                    NotePosition::RightOf => gaps[p] = gaps[p].max(width),
                    _ => {}
                }
            }
            SequenceEvent::FragmentStart { kind, label, .. } => {
                depth += 1;
                max_depth = max_depth.max(depth);
                frame_width = frame_width.max(measure(&format!("{kind:?}: {label}"))?.width + 80.0);
            }
            SequenceEvent::FragmentEnd { .. } => depth -= 1,
            SequenceEvent::Branch { label, .. } => {
                frame_width = frame_width.max(measure(label)?.width + 80.0)
            }
            _ => {}
        }
    }
    let margin = max_depth as f64 * 8.0;
    left += margin;
    right += margin;
    let mut x = left;
    let mut participants = Vec::new();
    for (i, &w) in widths.iter().enumerate() {
        participants.push(Rect {
            x,
            y: 20.0,
            width: w,
            height: header_height,
        });
        x += w;
        if i < n - 1 {
            x += gaps[i];
        }
    }
    let width = (x + right).max(frame_width + margin * 2.0);
    let centers: Vec<_> = participants.iter().map(|r| r.center().x).collect();
    let mut events = Vec::new();
    let mut activations = Vec::new();
    let mut active: Vec<Vec<(f64, f64)>> = vec![Vec::new(); n];
    let mut fragments = Vec::new();
    let mut y = header_height + 42.0;
    let mut number = 0;
    let mut last_message_y = None;
    for (i, e) in seq.events.iter().enumerate() {
        let at_x = |p: usize, other: usize| {
            let cx = centers[p] + active[p].len().saturating_sub(1) as f64 * 4.0;
            cx + if active[p].is_empty() || p == other {
                0.0
            } else if p < other {
                6.0
            } else {
                -6.0
            }
        };
        let geometry = match e {
            SequenceEvent::Message {
                source,
                target,
                label,
                ..
            } => {
                number += 1;
                let text = if seq.autonumber {
                    format!("{number}. {label}")
                } else {
                    label.clone()
                };
                let sz = measure(&text)?;
                let line_y = y + sz.height + 10.0;
                let a = at_x(*source, *target);
                let b = at_x(*target, *source);
                let (points, cx) = if source == target {
                    let outer = a + (sz.width + 30.0).max(70.0);
                    (
                        vec![
                            Point { x: a, y: line_y },
                            Point {
                                x: outer,
                                y: line_y,
                            },
                            Point {
                                x: outer,
                                y: line_y + 24.0,
                            },
                            Point {
                                x: b,
                                y: line_y + 24.0,
                            },
                        ],
                        (a + outer) / 2.0,
                    )
                } else {
                    (
                        vec![Point { x: a, y: line_y }, Point { x: b, y: line_y }],
                        (a + b) / 2.0,
                    )
                };
                let label = Rect {
                    x: cx - sz.width / 2.0,
                    y,
                    width: sz.width,
                    height: sz.height,
                };
                y = line_y + if source == target { 48.0 } else { 28.0 };
                EventGeometry::Message { points, label }
            }
            SequenceEvent::Note {
                participants: ps,
                position,
                label,
                ..
            } => {
                let sz = measure(label)?;
                let w = sz.width + 24.0;
                let min = ps.iter().map(|&p| centers[p]).fold(f64::INFINITY, f64::min);
                let max = ps
                    .iter()
                    .map(|&p| centers[p])
                    .fold(f64::NEG_INFINITY, f64::max);
                let x = match position {
                    NotePosition::Over => (min + max - w) / 2.0,
                    NotePosition::LeftOf => min - w - 18.0,
                    NotePosition::RightOf => max + 18.0,
                };
                let r = Rect {
                    x,
                    y,
                    width: w,
                    height: sz.height + 24.0,
                };
                y += r.height + 20.0;
                EventGeometry::Note(r)
            }
            SequenceEvent::Activate { participant, .. } => {
                let shift = active[*participant].len() as f64 * 4.0;
                active[*participant].push((
                    last_message_y.unwrap_or(y),
                    centers[*participant] + shift - 6.0,
                ));
                EventGeometry::Empty
            }
            SequenceEvent::Deactivate { participant, .. } => {
                let (start, x) = active[*participant].pop().unwrap();
                activations.push(Activation {
                    participant: *participant,
                    bounds: Rect {
                        x,
                        y: start,
                        width: 12.0,
                        height: (last_message_y.unwrap_or(y) - start).max(12.0),
                    },
                });
                EventGeometry::Empty
            }
            SequenceEvent::FragmentStart { kind, label, .. } => {
                let d = fragments.len() as f64 * 8.0;
                let text = format!("{kind:?}: {label}");
                let sz = measure(&text)?;
                let bounds = Rect {
                    x: 20.0 + d,
                    y,
                    width: width - 40.0 - 2.0 * d,
                    height: 0.0,
                };
                let label = Rect {
                    x: bounds.x + 12.0,
                    y: y + 10.0,
                    width: sz.width,
                    height: sz.height,
                };
                fragments.push(i);
                y += sz.height + 32.0;
                EventGeometry::Fragment { bounds, label }
            }
            SequenceEvent::Branch { label, .. } => {
                let sz = measure(label)?;
                let at = fragments.last().copied().unwrap();
                let EventGeometry::Fragment { bounds, .. } = &events[at] else {
                    unreachable!()
                };
                let label = Rect {
                    x: bounds.x + 12.0,
                    y: y + 10.0,
                    width: sz.width,
                    height: sz.height,
                };
                let line = y;
                y += sz.height + 32.0;
                EventGeometry::Branch { y: line, label }
            }
            SequenceEvent::FragmentEnd { .. } => {
                let start = fragments.pop().unwrap();
                y += 12.0;
                let EventGeometry::Fragment { bounds, .. } = &mut events[start] else {
                    unreachable!()
                };
                bounds.height = y - bounds.y;
                y += 12.0;
                EventGeometry::Empty
            }
        };
        if let EventGeometry::Message { points, .. } = &geometry {
            last_message_y = Some(points.last().unwrap().y);
        } else if !matches!(
            e,
            SequenceEvent::Activate { .. } | SequenceEvent::Deactivate { .. }
        ) {
            last_message_y = None;
        }
        events.push(geometry);
    }
    let lifeline_bottom = y + 12.0;
    Ok(SequenceLayout {
        participants,
        events,
        activations,
        size: Size {
            width,
            height: lifeline_bottom + header_height + 40.0,
        },
        lifeline_bottom,
    })
}
