use diagram_core::*;
use diagram_layout::layout;
use diagram_parser::parse;
use diagram_render::{Font, render, svg};
const FIXTURES: &[(&str, &str)] = &[
    ("branch", include_str!("../../../tests/fixtures/branch.mmd")),
    ("cycle", include_str!("../../../tests/fixtures/cycle.mmd")),
    ("groups", include_str!("../../../tests/fixtures/groups.mmd")),
    ("styles", include_str!("../../../tests/fixtures/styles.mmd")),
    (
        "escaping",
        include_str!("../../../tests/fixtures/escaping.mmd"),
    ),
    ("chain", include_str!("../../../tests/fixtures/chain.mmd")),
    ("hero", include_str!("../../../examples/hero.mmd")),
    (
        "parallel",
        include_str!("../../../tests/fixtures/parallel.mmd"),
    ),
    ("nested", include_str!("../../../tests/fixtures/nested.mmd")),
    (
        "extended",
        include_str!("../../../tests/fixtures/extended.mmd"),
    ),
];
fn intersects(a: Point, b: Point, r: Rect) -> bool {
    // Routes are orthogonal. Only intersections with the open rectangle count.
    let eps = 0.01;
    if (a.x - b.x).abs() < eps {
        a.x > r.x + eps
            && a.x < r.x + r.width - eps
            && a.y.min(b.y) < r.y + r.height - eps
            && a.y.max(b.y) > r.y + eps
    } else if (a.y - b.y).abs() < eps {
        a.y > r.y + eps
            && a.y < r.y + r.height - eps
            && a.x.min(b.x) < r.x + r.width - eps
            && a.x.max(b.x) > r.x + eps
    } else {
        panic!("non-orthogonal route {a:?} {b:?}")
    }
}
fn assert_geometry(g: &Graph) {
    let l = layout(g, &Font::default()).unwrap();
    assert!(l.size.width.is_finite() && l.size.height.is_finite());
    for (i, &a) in l.nodes.iter().enumerate() {
        assert!(
            a.x >= 0.0
                && a.y >= 0.0
                && a.x + a.width <= l.size.width
                && a.y + a.height <= l.size.height
        );
        for &b in &l.nodes[i + 1..] {
            assert!(!a.overlaps(b), "overlap {a:?} {b:?}");
        }
    }
    for (i, e) in l.edges.iter().enumerate() {
        let check_port = |end: Point, outside: Point, r: Rect| {
            let eps = 0.01;
            assert!(
                ((end.x - r.x).abs() < eps && outside.x < end.x && (outside.y - end.y).abs() < eps)
                    || ((end.x - r.x - r.width).abs() < eps
                        && outside.x > end.x
                        && (outside.y - end.y).abs() < eps)
                    || ((end.y - r.y).abs() < eps
                        && outside.y < end.y
                        && (outside.x - end.x).abs() < eps)
                    || ((end.y - r.y - r.height).abs() < eps
                        && outside.y > end.y
                        && (outside.x - end.x).abs() < eps),
                "non-normal port {end:?} {outside:?} {r:?}"
            );
        };
        check_port(e.points[0], e.points[1], l.nodes[g.edges[i].source]);
        check_port(
            *e.points.last().unwrap(),
            e.points[e.points.len() - 2],
            l.nodes[g.edges[i].target],
        );
        for (j, &r) in l.nodes.iter().enumerate() {
            if j == g.edges[i].source || j == g.edges[i].target {
                continue;
            }
            assert!(
                e.points.windows(2).all(|p| !intersects(p[0], p[1], r)),
                "edge {i} {:?} passes through node {j} {r:?}, points {:?}",
                g.edges[i],
                e.points
            );
        }
        if !g.edges[i].label.is_empty() {
            assert!(l.nodes.iter().all(|&r| !e.label.overlaps(r)));
        }
        for p in &e.points {
            assert!(p.x >= 0.0 && p.y >= 0.0 && p.x <= l.size.width && p.y <= l.size.height);
        }
    }
    for (i, &r) in l.groups.iter().enumerate() {
        let sz = Font::default().measure(&g.groups[i].label, 14.0).unwrap();
        let title = Rect {
            x: r.center().x - sz.width / 2.0,
            y: r.y + 17.0 - sz.height / 2.0,
            width: sz.width,
            height: sz.height,
        };
        for e in &l.edges {
            assert!(
                e.points.windows(2).all(|p| !intersects(p[0], p[1], title)),
                "route crosses group title"
            );
        }
        for (j, &a) in l.nodes.iter().enumerate() {
            if g.group_contains(g.nodes[j].group, i) {
                assert!(
                    a.x >= r.x
                        && a.y >= r.y
                        && a.x + a.width <= r.x + r.width
                        && a.y + a.height <= r.y + r.height
                );
            } else {
                assert!(!r.overlaps(a), "group includes unrelated node");
            }
        }
        for (j, &other) in l.groups.iter().enumerate().skip(i + 1) {
            if !g.group_contains(Some(i), j) && !g.group_contains(Some(j), i) {
                assert!(!r.overlaps(other), "unrelated groups overlap");
            }
        }
    }
}
#[test]
fn fixtures_satisfy_layout_invariants_in_every_direction() {
    for (_, s) in FIXTURES {
        for d in [Direction::TB, Direction::BT, Direction::LR, Direction::RL] {
            let mut g = parse(s).unwrap();
            g.direction = d;
            assert_geometry(&g);
        }
    }
}
#[test]
fn varied_graphs_have_no_node_overlaps_or_obstructed_edges() {
    let mut seed = 17_u64;
    for trial in 0..32 {
        let mut source = "flowchart TB\n".to_string();
        for i in 0..18 {
            source.push_str(&format!("N{i}[Node {i}]\n"));
        }
        for _ in 0..24 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let a = (seed >> 32) % 18;
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let b = (seed >> 32) % 18;
            source.push_str(&format!("N{a}-->N{b}\n"));
        }
        let mut g = parse(&source).unwrap();
        g.direction = [Direction::TB, Direction::BT, Direction::LR, Direction::RL][trial % 4];
        assert_geometry(&g);
    }
}
#[test]
fn crossing_sweeps_improve_a_crossed_layer() {
    let g = parse("flowchart TB; A-->D; B-->C; C; D").unwrap();
    let l = layout(&g, &Font::default()).unwrap();
    assert!(l.crossings_after <= l.crossings_before);
    assert_eq!(l.crossings_after, 0);
}
#[test]
fn cycle_reversal_does_not_change_ir_or_arrow_direction() {
    let g = parse("flowchart TB; A-->B-->C-->A").unwrap();
    let original = g.clone();
    let l = layout(&g, &Font::default()).unwrap();
    assert_eq!(g, original);
    assert!(l.edges.iter().any(|e| e.reversed_for_ranking));
    for (i, e) in l.edges.iter().enumerate() {
        let source = l.nodes[g.edges[i].source];
        let target = l.nodes[g.edges[i].target];
        let near = |p: Point, r: Rect| {
            p.x >= r.x - 0.01
                && p.x <= r.x + r.width + 0.01
                && p.y >= r.y - 0.01
                && p.y <= r.y + r.height + 0.01
        };
        assert!(near(e.points[0], source));
        assert!(near(*e.points.last().unwrap(), target));
    }
}
#[test]
fn font_metrics_are_shaped_and_multiline() {
    let f = Font::default();
    let narrow = f.measure("iiii", 15.0).unwrap();
    let wide = f.measure("WWWW", 15.0).unwrap();
    assert!(wide.width > narrow.width * 2.0);
    let multi = f.measure("WWWW\niiii", 15.0).unwrap();
    assert_eq!(multi.width, wide.width);
    assert_eq!(multi.height, wide.height * 2.0);
    assert_eq!(f.measure("漢字", 15.0).is_ok(), cfg!(feature = "cjk"));
    assert!(f.measure("\u{10ffff}", 15.0).is_err());
    assert!(Font::from_bytes(b"not a font").is_err());
    f.measure("café → مرحبا", 15.0).unwrap();
}
#[test]
fn svg_is_valid_xml_safe_and_deterministic() {
    for (_, s) in FIXTURES {
        let g = parse(s).unwrap();
        let a = render(&g).unwrap();
        assert_eq!(a, render(&g).unwrap());
        let doc = roxmltree::Document::parse(&a).unwrap();
        assert_eq!(doc.root_element().tag_name().name(), "svg");
        assert!(!a.contains("<script>"));
        assert!(
            doc.descendants()
                .filter_map(|n| n.attribute("href"))
                .all(|href| href.starts_with('#'))
        );
        assert!(a.contains("Bitstream"));
        assert_eq!(
            doc.descendants()
                .filter(|n| n.attribute("id").is_some_and(|id| id.starts_with("node-")))
                .count(),
            g.nodes.len()
        );
        assert_eq!(
            doc.descendants()
                .filter(|n| n.tag_name().name() == "marker")
                .count(),
            g.edges
                .iter()
                .filter(|e| e.directed || e.source_arrow)
                .count()
        );
    }
    let xml = render(&parse(FIXTURES[4].1).unwrap()).unwrap();
    assert!(xml.contains("&lt;script&gt;"));
    assert!(xml.contains("&amp;"));
}

#[test]
fn extended_links_enforce_rank_constraints_even_on_feedback_edges() {
    let mut g = parse(
        "flowchart TB; A([Start]) ---> B[[Process]] -..-> C[(Store)] <----> A; C <-->|loop| C",
    )
    .unwrap();
    for d in [Direction::TB, Direction::BT, Direction::LR, Direction::RL] {
        g.direction = d;
        assert_geometry(&g);
        let l = layout(&g, &Font::default()).unwrap();
        for e in &g.edges {
            if e.source != e.target {
                assert!(l.ranks[e.source].abs_diff(l.ranks[e.target]) >= e.min_rank_span);
            }
        }
        let xml = render(&g).unwrap();
        let doc = roxmltree::Document::parse(&xml).unwrap();
        for attr in ["marker-start", "marker-end"] {
            for url in doc.descendants().filter_map(|n| n.attribute(attr)) {
                let id = url
                    .strip_prefix("url(#")
                    .unwrap()
                    .strip_suffix(')')
                    .unwrap();
                let marker = doc
                    .descendants()
                    .find(|n| n.attribute("id") == Some(id))
                    .unwrap();
                assert_eq!(marker.tag_name().name(), "marker");
                if attr == "marker-start" {
                    assert_eq!(marker.attribute("orient"), Some("auto-start-reverse"));
                }
            }
        }
        assert_eq!(
            doc.descendants()
                .filter(|n| n.attribute("marker-start").is_some())
                .count(),
            2
        );
    }
}

#[test]
fn old_serialized_edges_default_to_single_rank_target_arrows() {
    let g = parse("flowchart TB; A --> B").unwrap();
    let mut value = serde_json::to_value(&g).unwrap();
    let edge = value["edges"][0].as_object_mut().unwrap();
    edge.remove("source_arrow");
    edge.remove("min_rank_span");
    assert_eq!(serde_json::from_value::<Graph>(value).unwrap(), g);
    for span in [0, 33, usize::MAX] {
        let mut invalid = g.clone();
        invalid.edges[0].min_rank_span = span;
        assert!(render(&invalid).is_err());
    }
}
#[test]
fn serialization_and_invalid_ir() {
    let g = parse(FIXTURES[0].1).unwrap();
    assert_eq!(
        g,
        serde_json::from_str::<Graph>(&serde_json::to_string(&g).unwrap()).unwrap()
    );
    let mut invalid = g.clone();
    invalid.edges[0].target = 999;
    assert!(render(&invalid).is_err());
    let mut invalid = g.clone();
    invalid.config.node_spacing = f64::NAN;
    assert!(render(&invalid).is_err());
    let mut invalid = g;
    invalid.nodes[0].style.fill = Some("url(https://bad.test)".into());
    assert!(render(&invalid).is_err());
}
#[test]
fn themes_have_distinct_backgrounds() {
    let mut g = parse(FIXTURES[0].1).unwrap();
    let light = render(&g).unwrap();
    g.config.theme = Theme::Dark;
    let dark = render(&g).unwrap();
    assert_ne!(light, dark);
    assert!(dark.contains("#0f172a"));
}
#[test]
fn svg_and_raster_visual_regressions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/snapshots");
    for (name, s) in FIXTURES {
        let g = parse(s).unwrap();
        let xml = svg(&g, &layout(&g, &Font::default()).unwrap(), &Font::default()).unwrap();
        assert_eq!(
            xml,
            std::fs::read_to_string(root.join(format!("{name}.svg"))).unwrap(),
            "SVG snapshot: {name}"
        );
        let tree = resvg::usvg::Tree::from_str(&xml, &resvg::usvg::Options::default()).unwrap();
        let size = tree.size().to_int_size();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );
        assert_eq!(
            pixmap.encode_png().unwrap(),
            std::fs::read(root.join(format!("{name}.png"))).unwrap(),
            "raster snapshot: {name}"
        );
    }
}

#[test]
fn long_group_titles_fit_and_remain_separate() {
    let mut g=parse("flowchart TB; subgraph a [Long descriptive client title]; A; end; subgraph b [Long descriptive server title]; B; end; A-->B").unwrap();
    for d in [Direction::TB, Direction::BT, Direction::LR, Direction::RL] {
        g.direction = d;
        assert_geometry(&g);
        let f = Font::default();
        let l = layout(&g, &f).unwrap();
        for (i, r) in l.groups.iter().enumerate() {
            let sz = f.measure(&g.groups[i].label, 14.0).unwrap();
            assert!(sz.width + 16.0 <= r.width);
            let title = Rect {
                x: r.center().x - sz.width / 2.0,
                y: r.y + 17.0 - sz.height / 2.0,
                width: sz.width,
                height: sz.height,
            };
            assert!(
                l.nodes.iter().all(|&n| !title.overlaps(n)),
                "{d:?}: group {r:?}, title {title:?}, nodes {:?}",
                l.nodes
            );
        }
    }
}

#[test]
fn different_inline_diagrams_have_distinct_fragment_ids() {
    let a = render(&parse("flowchart LR; A-->B").unwrap()).unwrap();
    let mut g = parse("flowchart LR; A-->B").unwrap();
    g.config.theme = Theme::Dark;
    let b = render(&g).unwrap();
    let a = roxmltree::Document::parse(&a).unwrap();
    let b = roxmltree::Document::parse(&b).unwrap();
    let ids: std::collections::BTreeSet<_> =
        a.descendants().filter_map(|n| n.attribute("id")).collect();
    assert!(
        b.descendants()
            .filter_map(|n| n.attribute("id"))
            .all(|id| !ids.contains(id))
    );
}
#[test]
fn parallel_and_opposite_edges_use_distinct_channels() {
    let mut g = parse(include_str!("../../../tests/fixtures/parallel.mmd")).unwrap();
    for d in [Direction::TB, Direction::BT, Direction::LR, Direction::RL] {
        g.direction = d;
        assert_geometry(&g);
        let l = layout(&g, &Font::default()).unwrap();
        for (i, e) in l.edges.iter().enumerate() {
            for other in &l.edges[i + 1..] {
                assert_ne!(e.points, other.points);
                let mut backwards = other.points.clone();
                backwards.reverse();
                assert_ne!(e.points, backwards);
            }
        }
    }
}
