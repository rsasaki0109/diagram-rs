#![cfg(feature = "cjk")]
use diagram_core::*;
use diagram_parser::parse_diagram;
use diagram_render::{Font, render_diagram, render_diagram_with_font};
#[test]
fn japanese_fallback_matches_metrics_and_embeds_license() {
    let f = Font::default();
    let jp = f.measure("日本語 ABC", 15.0).unwrap();
    assert!(jp.width > 60.0 && jp.height > 15.0);
    let d = parse_diagram(include_str!("../../../tests/fixtures/japanese.mmd")).unwrap();
    let xml = render_diagram(&d).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    assert!(xml.contains("SIL OPEN FONT LICENSE"));
    assert!(doc.descendants().any(|n| n.attribute("href").is_some()));
    let outlines = doc
        .descendants()
        .filter(|n| n.tag_name().name() == "path" && n.attribute("transform").is_some())
        .count();
    assert!(outlines > 0);
    let bytes = include_bytes!("../../../assets/fonts/DejaVuSans.ttf");
    let latin = Font::from_bytes(bytes).unwrap();
    assert!(render_diagram_with_font(&d, &latin).is_err());
}
#[test]
fn all_families_accept_japanese_labels() {
    for source in [
        "sequenceDiagram; actor A as 利用者; participant B as サーバー; A->>B: 注文; B-->>A: 完了",
        "classDiagram; class A[\"注文\"] {; +String 名前; +処理() bool; }",
        "stateDiagram-v2; state \"待機\" as A; [*]-->A; A-->[*]: 終了",
        "erDiagram; A ||--o{ B: 注文する; A {; int id PK \"主キー\"; }",
        "gantt; title 開発計画; section 実装; 構文解析: 2026-10-01, 3d",
    ] {
        let d = parse_diagram(source).unwrap();
        let xml = render_diagram(&d).unwrap();
        roxmltree::Document::parse(&xml).unwrap();
        assert!(xml.contains("SIL OPEN FONT LICENSE"));
    }
}
#[test]
fn japanese_svg_and_raster_regression() {
    let xml = render_diagram(
        &parse_diagram(include_str!("../../../tests/fixtures/japanese.mmd")).unwrap(),
    )
    .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/snapshots");
    assert_eq!(
        xml,
        std::fs::read_to_string(root.join("japanese.svg")).unwrap()
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
        std::fs::read(root.join("japanese.png")).unwrap()
    );
}
