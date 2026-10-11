use super::*;
use diagram_core::state::*;
pub(super) fn parse_state(source: &str) -> Result<StateDiagram, Diagnostic> {
    let (lines, config) = family_lines(source)?;
    let mut d = StateDiagram {
        config,
        ..StateDiagram::default()
    };
    let mut ids = BTreeMap::new();
    for (s, off) in lines.into_iter().skip(1) {
        let loc = span(source, off, s.len());
        if let Some(rest) = keyword(&s, "direction") {
            d.direction = direction(rest).map_err(|e| at(source, off, e))?;
            continue;
        }
        if let Some(rest) = keyword(&s, "state") {
            if rest.contains(['{', '}']) {
                return Err(at(source, off, "composite states are not yet supported"));
            }
            if rest.starts_with('"') {
                let at_alias = rest
                    .rfind(" as ")
                    .ok_or_else(|| at(source, off, "quoted state label requires as ID"))?;
                let label =
                    decode_label(rest[..at_alias].trim()).map_err(|e| at(source, off, e))?;
                let i = state_id(
                    rest[at_alias + 4..].trim(),
                    StateKind::Regular,
                    loc,
                    &mut d,
                    &mut ids,
                )?;
                d.states[i].label = label;
                continue;
            }
            let (id, kind) = if let Some((id, ann)) = rest.split_once(" <<") {
                let kind = match ann {
                    "choice>>" => StateKind::Choice,
                    "fork>>" => StateKind::Fork,
                    "join>>" => StateKind::Join,
                    _ => {
                        return Err(at(
                            source,
                            off,
                            "supported state stereotypes: choice, fork, join",
                        ));
                    }
                };
                (id.trim(), kind)
            } else {
                (rest, StateKind::Regular)
            };
            state_id(id, kind, loc, &mut d, &mut ids)?;
            continue;
        }
        if let Some((from, to)) = s.split_once("-->") {
            let (to, label) = to.split_once(':').unwrap_or((to, ""));
            let from = from.trim();
            let to = to.trim();
            let a = state_id(
                if from == "[*]" { "__initial" } else { from },
                if from == "[*]" {
                    StateKind::Initial
                } else {
                    StateKind::Regular
                },
                loc,
                &mut d,
                &mut ids,
            )?;
            let b = state_id(
                if to == "[*]" { "__final" } else { to },
                if to == "[*]" {
                    StateKind::Final
                } else {
                    StateKind::Regular
                },
                loc,
                &mut d,
                &mut ids,
            )?;
            if d.transitions.len() >= MAX_EDGES {
                return Err(at(source, off, "state diagram exceeds 2048 transitions"));
            }
            d.transitions.push(Transition {
                source: a,
                target: b,
                label: decode_label(label.trim()).map_err(|e| at(source, off, e))?,
                span: loc,
            });
            continue;
        }
        if let Some((id, desc)) = s.split_once(':') {
            let i = state_id(id.trim(), StateKind::Regular, loc, &mut d, &mut ids)?;
            let desc = decode_label(desc.trim()).map_err(|e| at(source, off, e))?;
            if !d.states[i].description.is_empty() {
                d.states[i].description.push('\n');
            }
            d.states[i].description.push_str(&desc);
            continue;
        }
        return Err(at(
            source,
            off,
            "unsupported state statement; expected state declaration or --> transition",
        ));
    }
    d.validate()?;
    Ok(d)
}
fn state_id(
    id: &str,
    kind: StateKind,
    loc: Span,
    d: &mut StateDiagram,
    ids: &mut BTreeMap<String, usize>,
) -> Result<usize, Diagnostic> {
    if matches!(id, "__initial" | "__final") && kind == StateKind::Regular {
        return Err(Diagnostic::new("reserved state identifier", loc));
    }
    validate_id(id).map_err(|e| Diagnostic::new(e, loc))?;
    if let Some(&i) = ids.get(id) {
        if kind != StateKind::Regular {
            if matches!(d.states[i].kind, StateKind::Initial | StateKind::Final)
                && d.states[i].kind != kind
            {
                return Err(Diagnostic::new("reserved state id collision", loc));
            }
            d.states[i].kind = kind;
        }
        return Ok(i);
    }
    if d.states.len() >= MAX_NODES {
        return Err(Diagnostic::new("state diagram exceeds 512 states", loc));
    }
    let i = d.states.len();
    d.states.push(State {
        id: id.into(),
        label: if matches!(kind, StateKind::Initial | StateKind::Final) {
            String::new()
        } else {
            id.into()
        },
        description: String::new(),
        kind,
        span: loc,
    });
    ids.insert(id.into(), i);
    Ok(i)
}
