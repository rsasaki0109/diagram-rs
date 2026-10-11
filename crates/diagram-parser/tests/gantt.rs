use diagram_core::{gantt::*, *};
use diagram_parser::parse_diagram;
#[test]
fn dates_and_leap_year_boundaries() {
    for date in [
        "0001-01-01",
        "1900-02-28",
        "2000-02-29",
        "2026-10-11",
        "9999-12-31",
    ] {
        let d = Date::parse(date).unwrap();
        assert_eq!(d.iso(), date);
    }
    assert_eq!(
        Date::parse("2000-02-28")
            .unwrap()
            .add_days(1)
            .unwrap()
            .iso(),
        "2000-02-29"
    );
    assert_eq!(
        Date::parse("1900-02-28")
            .unwrap()
            .add_days(1)
            .unwrap()
            .iso(),
        "1900-03-01"
    );
    for date in [
        "1900-02-29",
        "2026-02-30",
        "2026-13-01",
        "0000-01-01",
        "2026-1-01",
        "２０２６-１０-１１",
    ] {
        assert!(Date::parse(date).is_err());
    }
    assert!(Date::parse("9999-12-31").unwrap().add_days(1).is_err());
}
#[test]
fn gantt_resolves_dependencies_sections_statuses_and_milestones() {
    let Diagram::Gantt(g) =
        parse_diagram(include_str!("../../../tests/fixtures/gantt.mmd")).unwrap()
    else {
        panic!()
    };
    assert_eq!(g.tasks.len(), 6);
    assert_eq!(g.sections.len(), 2);
    assert_eq!(g.tasks[0].status, TaskStatus::Done);
    assert_eq!(g.tasks[1].status, TaskStatus::Active);
    assert!(g.tasks[2].critical);
    assert_eq!(g.tasks[4].start.iso(), "2026-10-13");
    assert_eq!(g.tasks[5].end.iso(), "2026-10-17");
    assert_eq!(g.tasks[4].dependencies, [2, 3]);
    assert!(g.tasks[5].milestone);
}
#[test]
fn gantt_forward_references_and_implicit_previous() {
    let Diagram::Gantt(g) =
        parse_diagram("gantt; One : first, after last, 1w; Last : last, 2026-01-01, 2d; Next : 3d")
            .unwrap()
    else {
        panic!()
    };
    assert_eq!(g.tasks[0].start.iso(), "2026-01-03");
    assert_eq!(g.tasks[0].end.iso(), "2026-01-10");
    assert_eq!(g.tasks[2].start.iso(), "2026-01-03");
}
#[test]
fn rejects_invalid_dates_cycles_unknown_dependencies_and_config() {
    for body in [
        "Task: 3d",
        "dateFormat DD/MM/YYYY; T: 2026-01-01, 3d",
        "excludes weekends; T: 2026-01-01, 3d",
        "T: a, after b, 1d; U: b, after a, 2d",
        "T: after missing, 3d",
        "T: 2026-02-30, 3d",
        "T: 2026-01-03, 2026-01-01",
        "T: 2026-01-03, 0d",
        "T: 2026-01-03, 1h",
        "T: a, 2026-01-01, 3d; U: a, 2026-01-05, 3d",
    ] {
        assert!(
            parse_diagram(&format!("gantt; {body}")).is_err(),
            "accepted {body}"
        );
    }
}
#[test]
fn gantt_ir_date_and_dependency_validation() {
    let mut d = parse_diagram(include_str!("../../../tests/fixtures/gantt.mmd")).unwrap();
    let Diagram::Gantt(g) = &mut d else { panic!() };
    g.tasks[0].dependencies.push(999);
    assert!(d.validate().is_err());
    let mut d = parse_diagram(include_str!("../../../tests/fixtures/gantt.mmd")).unwrap();
    let Diagram::Gantt(g) = &mut d else { panic!() };
    g.tasks[0].start = serde_json::from_str::<Date>("-1").unwrap();
    assert!(d.validate().is_err());
}
