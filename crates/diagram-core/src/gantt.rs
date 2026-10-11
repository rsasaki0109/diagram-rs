//! Calendar-day Gantt IR with resolved dates and explicit dependency indices.
use crate::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Date(i32);
impl Date {
    pub fn parse(s: &str) -> Result<Self, String> {
        let b = s.as_bytes();
        if b.len() != 10
            || b[4] != b'-'
            || b[7] != b'-'
            || !b
                .iter()
                .enumerate()
                .all(|(i, b)| matches!(i, 4 | 7) || b.is_ascii_digit())
        {
            return Err("date must use YYYY-MM-DD".into());
        }
        let y: i32 = s[..4].parse().map_err(|_| "invalid year")?;
        let m: usize = s[5..7].parse().map_err(|_| "invalid month")?;
        let d: i32 = s[8..].parse().map_err(|_| "invalid day")?;
        if !(1..=9999).contains(&y) || !(1..=12).contains(&m) {
            return Err("date outside years 0001..9999".into());
        }
        let months = month_days(y);
        if d < 1 || d > months[m - 1] {
            return Err("invalid calendar day".into());
        }
        Ok(Self(
            before_year(y) + months[..m - 1].iter().sum::<i32>() + d - 1,
        ))
    }
    pub fn from_day(day: i32) -> Result<Self, String> {
        if !(0..=3652058).contains(&day) {
            Err("date outside years 0001..9999".into())
        } else {
            Ok(Self(day))
        }
    }
    pub fn day(self) -> i32 {
        self.0
    }
    pub fn add_days(self, days: i32) -> Result<Self, String> {
        let n = self.0.checked_add(days).ok_or("date overflow")?;
        Self::from_day(n)
    }
    pub fn iso(self) -> String {
        let mut lo = 1;
        let mut hi = 10000;
        while lo + 1 < hi {
            let m = (lo + hi) / 2;
            if before_year(m) <= self.0 {
                lo = m;
            } else {
                hi = m;
            }
        }
        let mut rem = self.0 - before_year(lo);
        let mut month = 1;
        for n in month_days(lo) {
            if rem < n {
                break;
            }
            rem -= n;
            month += 1;
        }
        format!("{lo:04}-{month:02}-{:02}", rem + 1)
    }
}
fn before_year(y: i32) -> i32 {
    let y = y - 1;
    365 * y + y / 4 - y / 100 + y / 400
}
fn month_days(y: i32) -> [i32; 12] {
    [
        31,
        if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            29
        } else {
            28
        },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ]
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Normal,
    Active,
    Done,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub label: String,
    pub section: Option<usize>,
    pub start: Date,
    pub end: Date,
    pub status: TaskStatus,
    pub critical: bool,
    pub milestone: bool,
    pub dependencies: Vec<usize>,
    pub span: Span,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Gantt {
    pub title: String,
    pub sections: Vec<String>,
    pub tasks: Vec<Task>,
    pub config: Config,
}
impl Gantt {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        validate_config(&self.config)?;
        let loc = Span {
            line: 1,
            column: 1,
            ..Span::default()
        };
        validate_text(&self.title, loc)?;
        if self.tasks.is_empty() || self.tasks.len() > MAX_NODES || self.sections.len() > MAX_NODES
        {
            return Err(Diagnostic::new(
                "Gantt requires 1..=512 tasks and at most 512 sections",
                loc,
            ));
        }
        for s in &self.sections {
            validate_text(s, loc)?;
        }
        let mut ids = BTreeSet::new();
        let mut dependencies = 0;
        for t in &self.tasks {
            if t.id.is_empty() || !ids.insert(&t.id) {
                return Err(Diagnostic::new("empty or duplicate task id", t.span));
            }
            for text in [&t.id, &t.label] {
                validate_text(text, t.span)?;
            }
            if t.section.is_some_and(|s| s >= self.sections.len()) {
                return Err(Diagnostic::new("invalid task section", t.span));
            }
            if Date::from_day(t.start.day()).is_err()
                || Date::from_day(t.end.day()).is_err()
                || t.end < t.start
                || (!t.milestone && t.end == t.start)
            {
                return Err(Diagnostic::new("invalid task date range", t.span));
            }
            for &d in &t.dependencies {
                if d >= self.tasks.len() || t.start < self.tasks[d].end {
                    return Err(Diagnostic::new(
                        "invalid dependency or task starts before dependency ends",
                        t.span,
                    ));
                }
            }
            dependencies += t.dependencies.len();
        }
        if dependencies > MAX_EDGES {
            return Err(Diagnostic::new("Gantt exceeds 2048 dependencies", loc));
        }
        let min = self.tasks.iter().map(|t| t.start).min().unwrap();
        let max = self.tasks.iter().map(|t| t.end).max().unwrap();
        if max.day() - min.day() > 36600 {
            return Err(Diagnostic::new("Gantt date range exceeds 100 years", loc));
        }
        fn visit(i: usize, tasks: &[Task], color: &mut [u8]) -> bool {
            if color[i] == 1 {
                return false;
            }
            if color[i] == 2 {
                return true;
            }
            color[i] = 1;
            for &d in &tasks[i].dependencies {
                if !visit(d, tasks, color) {
                    return false;
                }
            }
            color[i] = 2;
            true
        }
        let mut color = vec![0; self.tasks.len()];
        for i in 0..self.tasks.len() {
            if !visit(i, &self.tasks, &mut color) {
                return Err(Diagnostic::new(
                    "cyclic task dependencies",
                    self.tasks[i].span,
                ));
            }
        }
        Ok(())
    }
}
