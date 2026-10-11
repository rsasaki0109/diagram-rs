use diagram_core::{sequence::*, *};
use diagram_layout::sequence::{EventGeometry, layout_sequence};
use diagram_parser::parse_diagram;
use diagram_render::{Font, render_diagram};
const SOURCE: &str = include_str!("../../../tests/fixtures/sequence.mmd");
fn check(s: &str) {
    let d = parse_diagram(s).unwrap();
    let Diagram::Sequence(seq) = &d else { panic!() };
    let l = layout_sequence(seq, &Font::default()).unwrap();
    let inside = |r: Rect| {
        assert!(
            r.x >= 0.0
                && r.y >= 0.0
                && r.width >= 0.0
                && r.height >= 0.0
                && r.x + r.width <= l.size.width + 0.01
                && r.y + r.height <= l.size.height + 0.01,
            "{r:?} {:?}",
            l.size
        );
    };
    for &r in &l.participants {
        inside(r);
    }
    for (i, e) in l.events.iter().enumerate() {
        match e {
            EventGeometry::Message { points, label } => {
                inside(*label);
                for p in points {
                    inside(Rect {
                        x: p.x,
                        y: p.y,
                        ..Rect::default()
                    });
                }
                if let SequenceEvent::Message { source, target, .. } = &seq.events[i] {
                    assert_eq!(points.len(), if source == target { 4 } else { 2 });
                }
            }
            EventGeometry::Note(r) => inside(*r),
            EventGeometry::Fragment { bounds, label } => {
                inside(*bounds);
                inside(*label);
                assert!(label.y >= bounds.y && label.y + label.height <= bounds.y + bounds.height);
            }
            EventGeometry::Branch { label, .. } => inside(*label),
            _ => {}
        }
    }
    for a in &l.activations {
        inside(a.bounds);
    }
    let xml = render_diagram(&d).unwrap();
    assert_eq!(xml, render_diagram(&d).unwrap());
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let ids: std::collections::BTreeSet<_> = doc
        .descendants()
        .filter_map(|n| n.attribute("id"))
        .collect();
    for n in doc.descendants() {
        if let Some(h) = n.attribute("href") {
            assert!(ids.contains(h.strip_prefix('#').unwrap()));
        }
        if let Some(m) = n.attribute("marker-end") {
            assert!(ids.contains(m.strip_prefix("url(#").unwrap().strip_suffix(')').unwrap()));
        }
    }
}
#[test]
fn sequence_geometry_and_svg() {
    check(SOURCE);
    check(
        "sequenceDiagram; actor A as One<br/>Two<br/>Three; A->>A: Self<br/>message; Note over A: Note",
    );
    check(
        "sequenceDiagram; A->>B: start; Note over A,B: A very long note that needs space beyond the original participant gap; Note left of A: Long note at the left edge; Note right of B: Long note at the right edge",
    );
}
#[test]
fn nested_fragments_and_activations() {
    check(
        "sequenceDiagram; participant A; activate A; activate A; loop once; opt ready; A->>A: self; end; end; deactivate A; deactivate A",
    );
}
#[test]
fn sequence_raster_regression() {
    let xml = render_diagram(&parse_diagram(SOURCE).unwrap()).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/snapshots");
    assert_eq!(
        xml,
        std::fs::read_to_string(root.join("sequence.svg")).unwrap()
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
        std::fs::read(root.join("sequence.png")).unwrap()
    );
}
