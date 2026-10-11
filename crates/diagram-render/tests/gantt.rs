use diagram_core::*;
use diagram_layout::gantt::layout_gantt;
use diagram_parser::parse_diagram;
use diagram_render::{Font, render_diagram};
#[test]
fn gantt_geometry_and_day_scales() {
    for source in [
        include_str!("../../../tests/fixtures/gantt.mmd"),
        "gantt; Milestone: milestone, 9999-12-31, 0d",
        "gantt; Task: 2026-01-01, 1d",
    ] {
        let d = parse_diagram(source).unwrap();
        let Diagram::Gantt(g) = &d else { panic!() };
        let l = layout_gantt(g, &Font::default()).unwrap();
        for t in &l.tasks {
            for r in [t.label, t.bar] {
                assert!(
                    r.x >= 0.0
                        && r.y >= 0.0
                        && r.x + r.width <= l.size.width
                        && r.y + r.height <= l.size.height
                );
            }
        }
        let xml = render_diagram(&d).unwrap();
        assert_eq!(xml, render_diagram(&d).unwrap());
        let doc = roxmltree::Document::parse(&xml).unwrap();
        assert_eq!(
            doc.descendants()
                .filter(|n| n.attribute("data-task-id").is_some())
                .count(),
            g.tasks.len()
        );
    }
}
#[test]
fn gantt_svg_and_raster_regression() {
    let xml =
        render_diagram(&parse_diagram(include_str!("../../../tests/fixtures/gantt.mmd")).unwrap())
            .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/snapshots");
    assert_eq!(
        xml,
        std::fs::read_to_string(root.join("gantt.svg")).unwrap()
    );
    let tree = resvg::usvg::Tree::from_str(&xml, &resvg::usvg::Options::default()).unwrap();
    let sz = tree.size().to_int_size();
    let mut p = resvg::tiny_skia::Pixmap::new(sz.width(), sz.height()).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut p.as_mut(),
    );
    assert_eq!(
        p.encode_png().unwrap(),
        std::fs::read(root.join("gantt.png")).unwrap()
    );
}
