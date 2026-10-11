use super::*;
use diagram_core::gantt::*;
enum Start {
    Date(Date),
    After(Vec<String>),
}
enum End {
    Date(Date),
    Duration(i32),
}
struct Raw {
    id: String,
    label: String,
    section: Option<usize>,
    start: Start,
    end: End,
    status: TaskStatus,
    critical: bool,
    milestone: bool,
    span: Span,
}
pub(super) fn parse_gantt(source: &str) -> Result<Gantt, Diagnostic> {
    let (lines, config) = family_lines(source)?;
    let mut d = Gantt {
        config,
        ..Gantt::default()
    };
    let mut raw: Vec<Raw> = Vec::new();
    let mut section = None;
    for (s, off) in lines.into_iter().skip(1) {
        let loc = span(source, off, s.len());
        if let Some(t) = keyword(&s, "title") {
            d.title = decode_label(t).map_err(|e| at(source, off, e))?;
            continue;
        }
        if let Some(t) = keyword(&s, "dateFormat") {
            if t != "YYYY-MM-DD" {
                return Err(at(source, off, "supported Gantt dateFormat: YYYY-MM-DD"));
            }
            continue;
        }
        if let Some(t) = keyword(&s, "axisFormat") {
            if t != "%Y-%m-%d" {
                return Err(at(source, off, "supported Gantt axisFormat: %Y-%m-%d"));
            }
            continue;
        }
        if let Some(t) = keyword(&s, "todayMarker") {
            if t != "off" {
                return Err(at(source, off, "supported todayMarker: off"));
            }
            continue;
        }
        if let Some(t) = keyword(&s, "section") {
            if d.sections.len() >= MAX_NODES {
                return Err(at(source, off, "Gantt exceeds 512 sections"));
            }
            section = Some(d.sections.len());
            d.sections
                .push(decode_label(t).map_err(|e| at(source, off, e))?);
            continue;
        }
        let (label, spec) = s.split_once(':').ok_or_else(|| {
            at(
                source,
                off,
                "unsupported Gantt statement; expected task label : fields",
            )
        })?;
        if raw.len() >= MAX_NODES {
            return Err(at(source, off, "Gantt exceeds 512 tasks"));
        }
        let mut fields: Vec<_> = spec.split(',').map(str::trim).collect();
        let mut status = TaskStatus::Normal;
        let mut critical = false;
        let mut milestone = false;
        while let Some(&first) = fields.first() {
            match first {
                "done" => status = TaskStatus::Done,
                "active" => status = TaskStatus::Active,
                "crit" => critical = true,
                "milestone" => milestone = true,
                _ => break,
            };
            fields.remove(0);
        }
        let id = if fields.len() == 3 {
            fields.remove(0).to_string()
        } else {
            format!("task{}", raw.len())
        };
        validate_id(&id).map_err(|e| at(source, off, e))?;
        if fields.is_empty() || fields.len() > 2 {
            return Err(at(
                source,
                off,
                "task fields: [id,] start, end/duration; or duration after a previous task",
            ));
        }
        let end = fields.pop().unwrap();
        let end = if let Ok(date) = Date::parse(end) {
            End::Date(date)
        } else {
            let (number, mult) = if let Some(s) = end.strip_suffix('d') {
                (s, 1)
            } else if let Some(s) = end.strip_suffix('w') {
                (s, 7)
            } else {
                return Err(at(
                    source,
                    off,
                    "duration requires whole calendar days (d) or weeks (w)",
                ));
            };
            let n = number
                .parse::<i32>()
                .ok()
                .and_then(|n| n.checked_mul(mult))
                .filter(|&n| (0..=36600).contains(&n))
                .ok_or_else(|| at(source, off, "duration must be between 0 and 36600 days"))?;
            if n == 0 && !milestone {
                return Err(at(source, off, "zero duration requires milestone"));
            }
            End::Duration(n)
        };
        let start = if let Some(first) = fields.pop() {
            if let Some(after) = first.strip_prefix("after ") {
                let ids: Vec<_> = after.split_whitespace().map(str::to_owned).collect();
                if ids.is_empty() {
                    return Err(at(source, off, "after requires task IDs"));
                }
                for id in &ids {
                    validate_id(id).map_err(|e| at(source, off, e))?;
                }
                Start::After(ids)
            } else {
                Start::Date(Date::parse(first).map_err(|e| at(source, off, e))?)
            }
        } else {
            let previous = raw
                .last()
                .ok_or_else(|| at(source, off, "first task requires an explicit start date"))?;
            Start::After(vec![previous.id.clone()])
        };
        raw.push(Raw {
            id,
            label: decode_label(label.trim()).map_err(|e| at(source, off, e))?,
            section,
            start,
            end,
            status,
            critical,
            milestone,
            span: loc,
        });
    }
    let mut ids = BTreeMap::new();
    for (i, r) in raw.iter().enumerate() {
        if ids.insert(r.id.clone(), i).is_some() {
            return Err(Diagnostic::new("duplicate task id", r.span));
        }
    }
    let mut resolved = vec![None; raw.len()];
    let mut visiting = vec![false; raw.len()];
    for i in 0..raw.len() {
        resolve(i, &raw, &ids, &mut resolved, &mut visiting)?;
    }
    for (i, r) in raw.iter().enumerate() {
        let (start, end) = resolved[i].unwrap();
        let dependencies = match &r.start {
            Start::Date(_) => Vec::new(),
            Start::After(v) => v.iter().map(|id| ids[id]).collect(),
        };
        d.tasks.push(Task {
            id: r.id.clone(),
            label: r.label.clone(),
            section: r.section,
            start,
            end,
            status: r.status,
            critical: r.critical,
            milestone: r.milestone,
            dependencies,
            span: r.span,
        });
    }
    d.validate()?;
    Ok(d)
}
fn resolve(
    i: usize,
    raw: &[Raw],
    ids: &BTreeMap<String, usize>,
    out: &mut [Option<(Date, Date)>],
    visiting: &mut [bool],
) -> Result<(Date, Date), Diagnostic> {
    if let Some(r) = out[i] {
        return Ok(r);
    }
    let r = &raw[i];
    let fail = |s| Diagnostic::new(s, r.span);
    if visiting[i] {
        return Err(fail("cyclic task dependencies".into()));
    }
    visiting[i] = true;
    let start = match &r.start {
        Start::Date(d) => *d,
        Start::After(v) => {
            let mut dates = Vec::new();
            for id in v {
                let &j = ids
                    .get(id)
                    .ok_or_else(|| fail(format!("unknown task dependency: {id}")))?;
                dates.push(resolve(j, raw, ids, out, visiting)?.1);
            }
            *dates.iter().max().unwrap()
        }
    };
    let end = match r.end {
        End::Date(d) => d,
        End::Duration(n) => start.add_days(n).map_err(fail)?,
    };
    visiting[i] = false;
    out[i] = Some((start, end));
    Ok((start, end))
}
