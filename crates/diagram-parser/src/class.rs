use super::*;
use diagram_core::class::*;
pub(super) fn parse_class(source: &str) -> Result<ClassDiagram, Diagnostic> {
    let (lines, config) = family_lines(source)?;
    let mut d = ClassDiagram {
        config,
        ..ClassDiagram::default()
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
            if s.contains(['{', '}']) {
                return Err(at(
                    source,
                    off,
                    "use separate lines for class members and closing brace",
                ));
            }
            member(&s, loc, &mut d.classes[i])?;
            continue;
        }
        if let Some(rest) = keyword(&s, "direction") {
            d.direction = direction(rest).map_err(|e| at(source, off, e))?;
            continue;
        }
        if let Some(rest) = keyword(&s, "class") {
            let (spec, block) = if let Some(r) = rest.strip_suffix('{') {
                (r.trim(), true)
            } else {
                (rest, false)
            };
            let (id, label) = if let Some(i) = spec.find('[') {
                let label = spec[i..]
                    .strip_prefix('[')
                    .and_then(|v| v.strip_suffix(']'))
                    .ok_or_else(|| at(source, off, "invalid class label"))?;
                (
                    &spec[..i],
                    Some(decode_label(label).map_err(|e| at(source, off, e))?),
                )
            } else {
                (spec, None)
            };
            let i = class_id(id.trim(), loc, &mut d, &mut ids)?;
            if let Some(label) = label {
                d.classes[i].label = label;
            }
            if block {
                current = Some(i);
            }
            continue;
        }
        if let Some((id, value)) = s.split_once(':')
            && !id.contains(['-', '<', '>', '.', '*'])
        {
            let i = class_id(id.trim(), loc, &mut d, &mut ids)?;
            member(value.trim(), loc, &mut d.classes[i])?;
            continue;
        }
        let (link, label) = s.split_once(':').unwrap_or((&s, ""));
        let mut quote = false;
        let mut at_link = None;
        for (i, ch) in link.char_indices() {
            if ch == '"' && !escaped(link, i) {
                quote = !quote;
            }
            if !quote
                && matches!(ch, '<' | '>' | '-' | '.' | '*' | 'o')
                && i > 0
                && let Some((token, kind, reversed)) = tokens()
                    .into_iter()
                    .find(|(t, _, _)| link[i..].starts_with(t))
            {
                at_link = Some((i, token, kind, reversed));
                break;
            }
        }
        let (i, t, kind, reversed) = at_link
            .ok_or_else(|| at(source, off, "unsupported class statement or relationship"))?;
        let (a, ac) = endpoint(link[..i].trim(), true).map_err(|e| at(source, off, e))?;
        let (b, bc) =
            endpoint(link[i + t.len()..].trim(), false).map_err(|e| at(source, off, e))?;
        let a = class_id(&a, loc, &mut d, &mut ids)?;
        let b = class_id(&b, loc, &mut d, &mut ids)?;
        if d.relations.len() >= MAX_EDGES {
            return Err(at(source, off, "class diagram exceeds 2048 relationships"));
        }
        d.relations.push(ClassRelation {
            source: a,
            target: b,
            kind,
            reversed,
            dashed: t.contains('.'),
            source_cardinality: ac,
            target_cardinality: bc,
            label: decode_label(label.trim()).map_err(|e| at(source, off, e))?,
            span: loc,
        });
    }
    if current.is_some() {
        return Err(at(source, source.len(), "unclosed class body; expected }"));
    }
    d.validate()?;
    Ok(d)
}
fn tokens() -> [(&'static str, ClassRelationKind, bool); 14] {
    use ClassRelationKind::*;
    [
        ("<|--", Inheritance, true),
        ("--|>", Inheritance, false),
        ("<|..", Realization, true),
        ("..|>", Realization, false),
        ("*--", Composition, true),
        ("--*", Composition, false),
        ("o--", Aggregation, true),
        ("--o", Aggregation, false),
        ("<--", Association, true),
        ("-->", Association, false),
        ("<..", Dependency, true),
        ("..>", Dependency, false),
        ("--", Link, false),
        ("..", Link, false),
    ]
}
fn endpoint(s: &str, left: bool) -> Result<(String, String), String> {
    if !s.contains('"') {
        validate_id(s)?;
        return Ok((s.into(), String::new()));
    }
    let first = s.find('"').unwrap();
    let last = s.rfind('"').unwrap();
    if first == last {
        return Err("unclosed cardinality quote".into());
    }
    let cardinality = &s[first + 1..last];
    let id = if left {
        if !s[last + 1..].trim().is_empty() {
            return Err("unexpected text after cardinality".into());
        }
        s[..first].trim()
    } else {
        if !s[..first].trim().is_empty() {
            return Err("cardinality must precede target class".into());
        }
        s[last + 1..].trim()
    };
    validate_id(id)?;
    Ok((id.into(), cardinality.into()))
}
fn class_id(
    id: &str,
    loc: Span,
    d: &mut ClassDiagram,
    ids: &mut BTreeMap<String, usize>,
) -> Result<usize, Diagnostic> {
    validate_id(id).map_err(|e| Diagnostic::new(e, loc))?;
    if let Some(&i) = ids.get(id) {
        return Ok(i);
    }
    if d.classes.len() >= MAX_NODES {
        return Err(Diagnostic::new("class diagram exceeds 512 classes", loc));
    }
    let i = d.classes.len();
    d.classes.push(Class {
        id: id.into(),
        label: id.into(),
        annotation: None,
        attributes: Vec::new(),
        methods: Vec::new(),
        span: loc,
    });
    ids.insert(id.into(), i);
    Ok(i)
}
fn member(s: &str, loc: Span, c: &mut Class) -> Result<(), Diagnostic> {
    if let Some(a) = s.strip_prefix("<<").and_then(|s| s.strip_suffix(">>")) {
        c.annotation = Some(a.into());
        return Ok(());
    }
    if s.is_empty() {
        return Err(Diagnostic::new("empty class member", loc));
    }
    if c.attributes.len() + c.methods.len() >= 256 {
        return Err(Diagnostic::new("class exceeds 256 members", loc));
    }
    let text = decode_label(s).map_err(|e| Diagnostic::new(e, loc))?;
    if s.contains('(') {
        if !s.contains(')') {
            return Err(Diagnostic::new("unclosed class method", loc));
        }
        c.methods.push(text);
    } else {
        c.attributes.push(text);
    }
    Ok(())
}
