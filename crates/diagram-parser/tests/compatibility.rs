use diagram_core::*;
use diagram_parser::parse;

#[test]
fn directions_and_headers() {
    for (s, d) in [
        ("TB", Direction::TB),
        ("TD", Direction::TB),
        ("BT", Direction::BT),
        ("LR", Direction::LR),
        ("RL", Direction::RL),
    ] {
        for h in ["graph", "flowchart"] {
            assert_eq!(parse(&format!("{h} {s}\nA-->B")).unwrap().direction, d);
        }
    }
}
#[test]
fn shapes_labels_and_redefinitions() {
    let g = parse("flowchart TB; A[x]-->B(y)-->C{z}-->D((c)); A[\"new #amp; name\"]").unwrap();
    assert_eq!(
        g.nodes.iter().map(|n| n.shape).collect::<Vec<_>>(),
        [
            Shape::Rectangle,
            Shape::Rounded,
            Shape::Diamond,
            Shape::Circle
        ]
    );
    assert_eq!(g.nodes[0].label, "new & name");
    assert_eq!(g.edges.len(), 3);
}
#[test]
fn connectors_chains_and_fanout() {
    let g =
        parse("flowchart TB; A & B -->|yes| C & D; C --- D; D -.-> A; B ==> C; C === A; D -.- B")
            .unwrap();
    assert_eq!(g.edges.len(), 9);
    assert!(g.edges[..4].iter().all(|e| e.label == "yes"));
    assert!(!g.edges[4].directed);
    assert_eq!(g.edges[5].kind, LineKind::Dotted);
    assert_eq!(g.edges[6].kind, LineKind::Thick);
    assert_eq!(
        parse("graph LR; A -- label --> B").unwrap().edges[0].label,
        "label"
    );
}
#[test]
fn multiline_comments_and_semicolons() {
    let g=parse("%% heading\nflowchart TB\nA[\"one;two\nthree<br/>four\\nfive\"] --> B %% trailing\n%% ignored\nB[Done]").unwrap();
    assert_eq!(g.nodes[0].label, "one;two\nthree\nfour\nfive");
    assert_eq!(g.nodes.len(), 2);
}
#[test]
fn classes_styles_and_forward_references() {
    let g=parse("flowchart TB; class A red; style B fill:#fff; linkStyle 0 stroke:blue; A:::red-->B; classDef default fill:green; classDef red fill:red,color:white").unwrap();
    assert_eq!(g.nodes[0].style.fill.as_deref(), Some("red"));
    assert_eq!(g.nodes[1].style.fill.as_deref(), Some("#fff"));
    assert_eq!(g.edges[0].style.stroke.as_deref(), Some("blue"));
}
#[test]
fn subgraphs_and_global_references() {
    let g = parse("flowchart LR; subgraph g [Group]; A-->B; end; B-->C").unwrap();
    assert_eq!(g.groups[0].label, "Group");
    assert_eq!(g.nodes[1].group, Some(0));
    assert_eq!(g.nodes[2].group, None);
}
#[test]
fn strict_init_config() {
    let g=parse("%%{init: {\"theme\":\"dark\",\"flowchart\":{\"nodeSpacing\":42,\"rankSpacing\":99}}}%%\nflowchart TB\nA").unwrap();
    assert_eq!(g.config.theme, Theme::Dark);
    assert_eq!(g.config.node_spacing, 42.0);
    assert_eq!(g.config.rank_spacing, 99.0);
}
#[test]
fn diagnostics_have_unicode_aware_locations() {
    let e = parse("flowchart TB\nA[\"café\"] ~~ B").unwrap_err();
    assert_eq!(e.span.line, 2);
    assert_eq!(e.span.column, 11);
    assert!(e.message.contains("connector"));
}
#[test]
fn malformed_and_unsupported_syntax_is_rejected() {
    for s in [
        "sequenceDiagram\nA->>B: x",
        "flowchart ZZ; A",
        "flowchart TB; A[broken",
        "flowchart TB; A{{hexagon}}",
        "flowchart TB; A(((double circle)))",
        "flowchart TB; A[/parallelogram/]",
        "flowchart TB; A[\\parallelogram\\]",
        "flowchart TB; A[/trapezoid\\]",
        "flowchart TB; A[\\trapezoid/]",
        "flowchart TB; A-->B; click A href",
        "flowchart TB; subgraph g; subgraph h; A; end",
        "flowchart TB; A; class A absent",
        "flowchart TB; A; style Missing fill:red",
        "flowchart TB; A; linkStyle 2 stroke:red",
        "flowchart TB; subgraph empty; end",
        "flowchart TB; A; direction LR",
        "flowchart TB; A@{shape: rect}",
    ] {
        assert!(parse(s).is_err(), "accepted {s}");
    }
}
#[test]
fn dangerous_styles_and_unknown_configuration_are_rejected() {
    for style in [
        "fill:url(https://evil.test)",
        "fill:var(--x)",
        "stroke-width:NaN",
        "font-size:1000",
        "onclick:alert(1)",
    ] {
        assert!(parse(&format!("flowchart TB; A; style A {style}")).is_err());
    }
    assert!(parse("%%{init: {\"securityLevel\":\"loose\"}}%%\nflowchart TB; A").is_err());
}
#[test]
fn input_and_graph_limits() {
    assert!(parse(&"x".repeat(MAX_SOURCE_BYTES + 1)).is_err());
    let mut s = "flowchart TB\n".to_string();
    for i in 0..=MAX_NODES {
        s.push_str(&format!("N{i}\n"));
    }
    assert!(parse(&s).is_err());
}
#[test]
fn supported_fixture_corpus() {
    for text in [
        include_str!("../../../tests/fixtures/branch.mmd"),
        include_str!("../../../tests/fixtures/cycle.mmd"),
        include_str!("../../../tests/fixtures/groups.mmd"),
        include_str!("../../../tests/fixtures/styles.mmd"),
        include_str!("../../../tests/fixtures/escaping.mmd"),
        include_str!("../../../tests/fixtures/chain.mmd"),
        include_str!("../../../tests/fixtures/extended.mmd"),
        include_str!("../../../examples/hero.mmd"),
    ] {
        parse(text).unwrap();
    }
}

#[test]
fn compound_shapes_and_quoted_delimiters() {
    let g = parse(
        r#"flowchart LR; A(["café [start]"])-->B[["validate (input)"]]-->C[("store {data}")]"#,
    )
    .unwrap();
    assert_eq!(
        g.nodes.iter().map(|n| n.shape).collect::<Vec<_>>(),
        [Shape::Stadium, Shape::Subroutine, Shape::Cylinder]
    );
    assert_eq!(g.nodes[0].label, "café [start]");
    assert_eq!(g.nodes[1].label, "validate (input)");
    assert_eq!(g.nodes[2].label, "store {data}");
    for s in [
        "A([broken)",
        "A[[broken]",
        "A[(broken]]",
        "A([nested[bad]])",
    ] {
        assert!(
            parse(&format!("flowchart TB; {s}")).is_err(),
            "accepted {s}"
        );
    }
}

#[test]
fn arrowheads_and_link_lengths_survive_chains_and_fanout() {
    for (token, kind, directed, source_arrow, span) in [
        ("-->", LineKind::Solid, true, false, 1),
        ("--->", LineKind::Solid, true, false, 2),
        ("----", LineKind::Solid, false, false, 2),
        ("<-->", LineKind::Solid, true, true, 1),
        ("<---->", LineKind::Solid, true, true, 3),
        ("-..->", LineKind::Dotted, true, false, 2),
        ("-...-", LineKind::Dotted, false, false, 3),
        ("<-.->", LineKind::Dotted, true, true, 1),
        ("<-...->", LineKind::Dotted, true, true, 3),
        ("===>", LineKind::Thick, true, false, 2),
        ("====", LineKind::Thick, false, false, 2),
        ("<==>", LineKind::Thick, true, true, 1),
        ("<====>", LineKind::Thick, true, true, 3),
    ] {
        let g = parse(&format!("flowchart LR; A & B {token}|payload| C & D --> E")).unwrap();
        assert_eq!(g.edges.len(), 6);
        for edge in &g.edges[..4] {
            assert_eq!(
                (
                    edge.kind,
                    edge.directed,
                    edge.source_arrow,
                    edge.min_rank_span
                ),
                (kind, directed, source_arrow, span),
                "{token}"
            );
            assert_eq!(edge.label, "payload");
        }
    }
    let g = parse(r#"flowchart LR; A -- "label --> with arrows" ----> B"#).unwrap();
    assert_eq!(g.edges[0].label, "label --> with arrows");
    assert_eq!(g.edges[0].min_rank_span, 3);
    let g = parse(&format!("flowchart TB; A {}> B", "-".repeat(33))).unwrap();
    assert_eq!(g.edges[0].min_rank_span, 10);
    for token in [
        "<--", "<---", "--", "=", "-.>", "<-.-->", "-->>", "o--o", "x--x",
    ] {
        assert!(
            parse(&format!("flowchart TB; A {token} B")).is_err(),
            "accepted {token}"
        );
    }
    for token in [
        format!("{}>", "-".repeat(34)),
        "=".repeat(50),
        format!("-{}->", ".".repeat(20)),
    ] {
        assert_eq!(
            parse(&format!("flowchart TB; A {token} B")).unwrap().edges[0].min_rank_span,
            10
        );
    }
}

#[test]
fn multiple_class_assignments_combine() {
    let g=parse("flowchart TB; A; class A red; class A wide; classDef red fill:red; classDef wide stroke-width:4px").unwrap();
    assert_eq!(g.nodes[0].style.fill.as_deref(), Some("red"));
    assert_eq!(g.nodes[0].style.stroke_width, Some(4.0));
}

#[test]
fn quoted_backslashes_and_crlf_labels() {
    let g = parse(r#"flowchart TB; A["path\\"]-->B"#).unwrap();
    assert_eq!(g.nodes[0].label, r"path\\");
    let g = parse("flowchart TB\r\nA[\"one\r\ntwo\"]-->B\r\n").unwrap();
    assert_eq!(g.nodes[0].label, "one\ntwo");
}
#[test]
fn nested_subgraphs_preserve_parentage_and_leaf_membership() {
    let g = parse(include_str!("../../../tests/fixtures/nested.mmd")).unwrap();
    assert_eq!(g.groups.len(), 4);
    assert_eq!(g.groups[1].parent, Some(0));
    assert_eq!(g.groups[2].parent, Some(0));
    assert_eq!(g.groups[3].parent, None);
    assert!(g.group_contains(g.nodes[0].group, 0));
    assert!(g.group_contains(g.nodes[0].group, 1));
    assert!(!g.group_contains(g.nodes[0].group, 2));
    let g =
        parse("flowchart TB; subgraph outer; A; subgraph inner; A-->B; end; B-->C; end").unwrap();
    assert_eq!(g.nodes[0].group, Some(1));
    assert_eq!(g.nodes[1].group, Some(1));
    assert_eq!(g.nodes[2].group, Some(0));
    let mut invalid = g;
    invalid.groups[1].parent = Some(1);
    assert!(invalid.validate().is_err());
    let s = format!(
        "flowchart TB; {}A; {}",
        "subgraph g; ".repeat(33),
        "end; ".repeat(33)
    );
    assert!(parse(&s).is_err());
}
