use diagram_core::*;
use diagram_parser::parse_diagram;
use diagram_render::render_diagram;
const FIXTURES: &[(&str, &str)] = &[
    ("class", include_str!("../../../tests/fixtures/class.mmd")),
    ("state", include_str!("../../../tests/fixtures/state.mmd")),
    ("er", include_str!("../../../tests/fixtures/er.mmd")),
];
fn check_xml(xml: &str) {
    let d = roxmltree::Document::parse(xml).unwrap();
    let ids: std::collections::BTreeSet<_> =
        d.descendants().filter_map(|n| n.attribute("id")).collect();
    for n in d.descendants() {
        if let Some(h) = n.attribute("href") {
            assert!(ids.contains(h.strip_prefix('#').unwrap()));
        }
        for attr in ["marker-start", "marker-end"] {
            if let Some(m) = n.attribute(attr) {
                assert!(ids.contains(m.strip_prefix("url(#").unwrap().strip_suffix(')').unwrap()));
            }
        }
        if n.tag_name().name() == "rect" {
            for attr in ["width", "height"] {
                if let Some(s) = n.attribute(attr).filter(|s| !s.ends_with('%')) {
                    assert!(s.parse::<f64>().unwrap() >= 0.0);
                }
            }
        }
    }
    assert!(!xml.contains("<script>"));
}
#[test]
fn families_render_in_all_directions_and_themes() {
    for (_, s) in FIXTURES {
        for direction in [Direction::TB, Direction::BT, Direction::LR, Direction::RL] {
            let mut d = parse_diagram(s).unwrap();
            match &mut d {
                Diagram::Class(c) => c.direction = direction,
                Diagram::State(s) => s.direction = direction,
                Diagram::Er(e) => e.direction = direction,
                _ => unreachable!(),
            };
            for theme in [Theme::Light, Theme::Dark] {
                d.config_mut().theme = theme;
                let xml = render_diagram(&d).unwrap();
                assert_eq!(xml, render_diagram(&d).unwrap());
                check_xml(&xml);
            }
        }
    }
}
#[test]
fn family_svg_and_png_regressions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/snapshots");
    for (name, s) in FIXTURES {
        let xml = render_diagram(&parse_diagram(s).unwrap()).unwrap();
        assert_eq!(
            xml,
            std::fs::read_to_string(root.join(format!("{name}.svg"))).unwrap()
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
            std::fs::read(root.join(format!("{name}.png"))).unwrap()
        );
    }
}
#[test]
fn minimum_card_sizes_reject_nonfinite_and_mismatched_input() {
    let g = diagram_parser::parse("flowchart TB; A-->B").unwrap();
    for sizes in [
        vec![Size {
            width: 100.0,
            height: 100.0,
        }],
        vec![
            Size {
                width: f64::NAN,
                height: 100.0
            };
            2
        ],
        vec![
            Size {
                width: 0.0,
                height: 100.0
            };
            2
        ],
    ] {
        assert!(
            diagram_layout::layout_with_node_sizes(&g, &diagram_render::Font::default(), &sizes)
                .is_err()
        );
    }
}
