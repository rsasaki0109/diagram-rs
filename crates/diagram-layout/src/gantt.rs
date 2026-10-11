//! Calendar-day timeline positioning; no browser or system date is consulted.
use diagram_core::{gantt::*, *};
#[derive(Clone, Debug, PartialEq)]
pub struct TaskGeometry {
    pub label: Rect,
    pub bar: Rect,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SectionGeometry {
    pub section: usize,
    pub label: Rect,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GanttLayout {
    pub size: Size,
    pub tasks: Vec<TaskGeometry>,
    pub sections: Vec<SectionGeometry>,
    pub ticks: Vec<(Date, f64)>,
    pub axis_y: f64,
    pub title: Rect,
}
pub fn layout_gantt(g: &Gantt, m: &impl TextMeasure) -> Result<GanttLayout, Diagnostic> {
    g.validate()?;
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
    let measure = |s: &str| m.measure(s, 15.0).map_err(fail);
    let mut label_width = 220.0_f64;
    for t in &g.tasks {
        label_width = label_width.max(measure(&t.label)?.width + 30.0);
    }
    for s in &g.sections {
        label_width = label_width.max(measure(s)?.width + 30.0);
    }
    let title_size = measure(&g.title)?;
    let chart_width = 1000.0_f64.max(title_size.width - label_width + 40.0);
    let size_width = label_width + chart_width + 80.0;
    let min = g.tasks.iter().map(|t| t.start).min().unwrap();
    let max = g.tasks.iter().map(|t| t.end).max().unwrap();
    let days = (max.day() - min.day()).max(1);
    let scale = chart_width / days as f64;
    let title = Rect {
        x: (size_width - title_size.width) / 2.0,
        y: 20.0,
        width: title_size.width,
        height: title_size.height,
    };
    let axis_y = title.y + title.height + 55.0;
    let mut y = axis_y + 20.0;
    let mut tasks = Vec::new();
    let mut sections = Vec::new();
    let mut current = None;
    for t in &g.tasks {
        if t.section != current {
            if let Some(i) = t.section {
                let sz = measure(&g.sections[i])?;
                sections.push(SectionGeometry {
                    section: i,
                    label: Rect {
                        x: 20.0,
                        y,
                        width: sz.width,
                        height: sz.height,
                    },
                });
                y += sz.height + 20.0;
            }
            current = t.section;
        }
        let sz = measure(&t.label)?;
        let h = (sz.height + 16.0).max(34.0);
        let label = Rect {
            x: 20.0,
            y: y + (h - sz.height) / 2.0,
            width: sz.width,
            height: sz.height,
        };
        let x = label_width + 40.0 + (t.start.day() - min.day()) as f64 * scale;
        let bar = Rect {
            x,
            y: y + (h - 24.0) / 2.0,
            width: ((t.end.day() - t.start.day()) as f64 * scale).max(1.0),
            height: 24.0,
        };
        tasks.push(TaskGeometry { label, bar });
        y += h + 8.0;
    }
    let step = ((days as f64 / 8.0).ceil() as i32).max(1);
    let mut ticks = Vec::new();
    let mut day = 0;
    while day <= max.day() - min.day() {
        ticks.push((
            min.add_days(day).map_err(fail)?,
            label_width + 40.0 + day as f64 * scale,
        ));
        day += step;
    }
    Ok(GanttLayout {
        size: Size {
            width: size_width + 12.0,
            height: y + 24.0,
        },
        tasks,
        sections,
        ticks,
        axis_y,
        title,
    })
}
