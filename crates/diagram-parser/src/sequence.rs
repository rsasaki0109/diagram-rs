use super::*;
use diagram_core::sequence::*;

pub(super) fn parse_sequence(source: &str) -> Result<Sequence, Diagnostic> {
    let (lines, config) = family_lines(source)?;
    let mut seq = Sequence {
        config,
        ..Sequence::default()
    };
    let mut ids = BTreeMap::new();
    let mut scopes = Vec::new();
    for (s, off) in lines.into_iter().skip(1) {
        let loc = span(source, off, s.len());
        if seq.events.len() >= MAX_EDGES {
            return Err(at(source, off, "sequence exceeds 2048 events"));
        }
        if s == "autonumber" {
            seq.autonumber = true;
            continue;
        }
        if let Some((rest, kind)) = keyword(&s, "participant")
            .map(|r| (r, ParticipantKind::Participant))
            .or_else(|| keyword(&s, "actor").map(|r| (r, ParticipantKind::Actor)))
        {
            let (id, label) = rest.split_once(" as ").unwrap_or((rest, rest));
            let p = participant(id.trim(), loc, &mut seq, &mut ids)?;
            seq.participants[p].label =
                decode_label(label.trim()).map_err(|e| at(source, off, e))?;
            seq.participants[p].kind = kind;
            continue;
        }
        if let Some((rest, activate)) = keyword(&s, "activate")
            .map(|r| (r, true))
            .or_else(|| keyword(&s, "deactivate").map(|r| (r, false)))
        {
            let p = participant(rest, loc, &mut seq, &mut ids)?;
            seq.events.push(if activate {
                SequenceEvent::Activate {
                    participant: p,
                    span: loc,
                }
            } else {
                SequenceEvent::Deactivate {
                    participant: p,
                    span: loc,
                }
            });
            continue;
        }
        if let Some(rest) = s.strip_prefix("Note ").or_else(|| s.strip_prefix("note ")) {
            let (who, label) = rest
                .split_once(':')
                .ok_or_else(|| at(source, off, "note requires : label"))?;
            let (who, position) = if let Some(who) = who.strip_prefix("left of ") {
                (who, NotePosition::LeftOf)
            } else if let Some(who) = who.strip_prefix("right of ") {
                (who, NotePosition::RightOf)
            } else if let Some(who) = who.strip_prefix("over ") {
                (who, NotePosition::Over)
            } else {
                return Err(at(source, off, "note requires left of, right of, or over"));
            };
            let mut ps = Vec::new();
            for id in who.split(',') {
                ps.push(participant(id.trim(), loc, &mut seq, &mut ids)?);
            }
            seq.events.push(SequenceEvent::Note {
                participants: ps,
                position,
                label: decode_label(label.trim()).map_err(|e| at(source, off, e))?,
                span: loc,
            });
            continue;
        }
        if s == "end" {
            if scopes.pop().is_none() {
                return Err(at(source, off, "unexpected sequence end"));
            }
            seq.events.push(SequenceEvent::FragmentEnd { span: loc });
            continue;
        }
        let mut control = false;
        for (name, kind) in [
            ("loop", FragmentKind::Loop),
            ("alt", FragmentKind::Alt),
            ("opt", FragmentKind::Opt),
            ("par", FragmentKind::Par),
            ("critical", FragmentKind::Critical),
            ("break", FragmentKind::Break),
        ] {
            if s == name || keyword(&s, name).is_some() {
                let label = keyword(&s, name).unwrap_or("");
                if scopes.len() >= 32 {
                    return Err(at(source, off, "fragment depth exceeds 32"));
                }
                scopes.push(kind);
                seq.events.push(SequenceEvent::FragmentStart {
                    kind,
                    label: decode_label(label).map_err(|e| at(source, off, e))?,
                    span: loc,
                });
                control = true;
                break;
            }
        }
        if control {
            continue;
        }
        for (name, kind) in [
            ("else", FragmentKind::Alt),
            ("and", FragmentKind::Par),
            ("option", FragmentKind::Critical),
        ] {
            if s == name || keyword(&s, name).is_some() {
                if scopes.last() != Some(&kind) {
                    return Err(at(
                        source,
                        off,
                        "branch does not match its enclosing fragment",
                    ));
                }
                seq.events.push(SequenceEvent::Branch {
                    label: decode_label(keyword(&s, name).unwrap_or(""))
                        .map_err(|e| at(source, off, e))?,
                    span: loc,
                });
                control = true;
                break;
            }
        }
        if control {
            continue;
        }
        let (link, label) = s.split_once(':').ok_or_else(|| {
            at(
                source,
                off,
                "unsupported sequence statement; expected a message, declaration, note or fragment",
            )
        })?;
        let start = link
            .find(['-', '>'])
            .ok_or_else(|| at(source, off, "expected sequence arrow"))?;
        let from = link[..start].trim();
        let rest = &link[start..];
        let (token, kind) = [
            ("-->>", MessageKind::DottedArrow),
            ("->>", MessageKind::SolidArrow),
            ("--x", MessageKind::DottedCross),
            ("-x", MessageKind::SolidCross),
            ("-->", MessageKind::DottedOpen),
            ("->", MessageKind::SolidOpen),
        ]
        .into_iter()
        .find(|(t, _)| rest.starts_with(t))
        .ok_or_else(|| at(source, off + start, "unsupported sequence arrow"))?;
        let mut target = rest[token.len()..].trim();
        let activation = target.chars().next().filter(|c| matches!(c, '+' | '-'));
        if activation.is_some() {
            target = target[1..].trim();
        }
        let a = participant(from, loc, &mut seq, &mut ids)?;
        let b = participant(target, loc, &mut seq, &mut ids)?;
        seq.events.push(SequenceEvent::Message {
            source: a,
            target: b,
            label: decode_label(label.trim()).map_err(|e| at(source, off, e))?,
            kind,
            span: loc,
        });
        match activation {
            Some('+') => seq.events.push(SequenceEvent::Activate {
                participant: b,
                span: loc,
            }),
            Some('-') => seq.events.push(SequenceEvent::Deactivate {
                participant: a,
                span: loc,
            }),
            _ => {}
        }
        if seq.events.len() > MAX_EDGES {
            return Err(at(source, off, "sequence exceeds 2048 events"));
        }
    }
    seq.validate()?;
    Ok(seq)
}
fn participant(
    id: &str,
    loc: Span,
    seq: &mut Sequence,
    ids: &mut BTreeMap<String, usize>,
) -> Result<usize, Diagnostic> {
    validate_id(id).map_err(|e| Diagnostic::new(e, loc))?;
    if let Some(&p) = ids.get(id) {
        return Ok(p);
    }
    if seq.participants.len() >= MAX_NODES {
        return Err(Diagnostic::new("sequence exceeds 512 participants", loc));
    }
    let p = seq.participants.len();
    seq.participants.push(Participant {
        id: id.into(),
        label: id.into(),
        kind: ParticipantKind::Participant,
        span: loc,
    });
    ids.insert(id.into(), p);
    Ok(p)
}
