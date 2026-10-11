use super::*;
use diagram_core::er::*;
pub(super) fn parse_er(source: &str) -> Result<ErDiagram, Diagnostic> {
    let (lines, config) = family_lines(source)?;
    let mut d = ErDiagram {
        config,
        ..ErDiagram::default()
    };
    let mut ids = BTreeMap::new();
    let mut current: Option<usize> = None;
    for (s, off) in lines.into_iter().skip(1) {
        let loc = span(source, off, s.len());
        if let Some(i) = current {
            if s == "}" {
                current = None;
                continue;
            }
            if d.entities[i].attributes.len() >= 256 {
                return Err(at(source, off, "entity exceeds 256 attributes"));
            }
            d.entities[i].attributes.push(attribute(&s, loc)?);
            continue;
        }
        if let Some(rest) = keyword(&s, "direction") {
            d.direction = direction(rest).map_err(|e| at(source, off, e))?;
            continue;
        }
        if let Some(id) = s.strip_suffix('{') {
            let i = entity(id.trim(), loc, &mut d, &mut ids)?;
            current = Some(i);
            continue;
        }
        if !s.contains(['|', '}', ':', '-', '.']) {
            entity(&s, loc, &mut d, &mut ids)?;
            continue;
        }
        let (link, label) = s
            .split_once(':')
            .ok_or_else(|| at(source, off, "ER relationship requires : label"))?;
        let (idx, token) = link
            .find("--")
            .map(|i| (i, "--"))
            .or_else(|| link.find("..").map(|i| (i, "..")))
            .ok_or_else(|| {
                at(
                    source,
                    off,
                    "expected identifying -- or non-identifying .. relationship",
                )
            })?;
        let left = link[..idx].trim();
        let right = link[idx + 2..].trim();
        if left.len() < 2
            || right.len() < 2
            || !left.is_char_boundary(left.len() - 2)
            || !right.is_char_boundary(2)
        {
            return Err(at(source, off, "invalid ER cardinality"));
        }
        let ac = cardinality(&left[left.len() - 2..], true).map_err(|e| at(source, off, e))?;
        let bc = cardinality(&right[..2], false).map_err(|e| at(source, off, e))?;
        let a = entity(left[..left.len() - 2].trim(), loc, &mut d, &mut ids)?;
        let b = entity(right[2..].trim(), loc, &mut d, &mut ids)?;
        if d.relationships.len() >= MAX_EDGES {
            return Err(at(source, off, "ER diagram exceeds 2048 relationships"));
        }
        d.relationships.push(Relationship {
            source: a,
            target: b,
            source_cardinality: ac,
            target_cardinality: bc,
            identifying: token == "--",
            label: decode_label(label.trim()).map_err(|e| at(source, off, e))?,
            span: loc,
        });
    }
    if current.is_some() {
        return Err(at(source, source.len(), "unclosed entity body; expected }"));
    }
    d.validate()?;
    Ok(d)
}
fn cardinality(s: &str, left: bool) -> Result<Cardinality, String> {
    use Cardinality::*;
    match (s, left) {
        ("||", _) => Ok(ExactlyOne),
        ("|o", true) | ("o|", false) => Ok(ZeroOrOne),
        ("}|", true) | ("|{", false) => Ok(OneOrMore),
        ("}o", true) | ("o{", false) => Ok(ZeroOrMore),
        _ => Err("unsupported ER cardinality; use ||, |o/o|, }|/|{ or }o/o{".into()),
    }
}
fn entity(
    id: &str,
    loc: Span,
    d: &mut ErDiagram,
    ids: &mut BTreeMap<String, usize>,
) -> Result<usize, Diagnostic> {
    validate_id(id).map_err(|e| Diagnostic::new(e, loc))?;
    if let Some(&i) = ids.get(id) {
        return Ok(i);
    }
    if d.entities.len() >= MAX_NODES {
        return Err(Diagnostic::new("ER diagram exceeds 512 entities", loc));
    }
    let i = d.entities.len();
    d.entities.push(Entity {
        id: id.into(),
        attributes: Vec::new(),
        span: loc,
    });
    ids.insert(id.into(), i);
    Ok(i)
}
fn attribute(s: &str, loc: Span) -> Result<Attribute, Diagnostic> {
    let (body, comment) = if let Some(i) = s.find('"') {
        (
            &s[..i],
            decode_label(s[i..].trim()).map_err(|e| Diagnostic::new(e, loc))?,
        )
    } else {
        (s, String::new())
    };
    let mut words = body.split_whitespace();
    let data_type = words
        .next()
        .ok_or_else(|| Diagnostic::new("attribute requires type and name", loc))?;
    let name = words
        .next()
        .ok_or_else(|| Diagnostic::new("attribute requires type and name", loc))?;
    validate_id(name).map_err(|e| Diagnostic::new(e, loc))?;
    if !data_type
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'[' | b']'))
    {
        return Err(Diagnostic::new("unsupported attribute type", loc));
    }
    let keytext = words.collect::<Vec<_>>().join("");
    let mut keys = Vec::new();
    if !keytext.is_empty() {
        for k in keytext.split(',') {
            keys.push(match k {
                "PK" => AttributeKey::PK,
                "FK" => AttributeKey::FK,
                "UK" => AttributeKey::UK,
                _ => return Err(Diagnostic::new("attribute keys must be PK, FK or UK", loc)),
            });
        }
    }
    Ok(Attribute {
        data_type: data_type.into(),
        name: name.into(),
        keys,
        comment,
        span: loc,
    })
}
