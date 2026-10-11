use diagram_core::{class::*, er::*, state::*, *};
use diagram_parser::parse_diagram;
#[test]
fn class_compartments_and_relationships() {
    let Diagram::Class(c) =
        parse_diagram(include_str!("../../../tests/fixtures/class.mmd")).unwrap()
    else {
        panic!()
    };
    assert_eq!(c.classes.len(), 4);
    assert_eq!(c.relations.len(), 3);
    assert_eq!(c.classes[0].annotation.as_deref(), Some("interface"));
    assert_eq!(c.classes[0].methods, ["+pay(amount) bool"]);
    assert_eq!(c.relations[0].kind, ClassRelationKind::Realization);
    assert!(c.relations[0].dashed && c.relations[0].reversed);
    assert_eq!(c.relations[2].source_cardinality, "1");
    assert_eq!(c.relations[2].target_cardinality, "1..*");
}
#[test]
fn class_arrow_matrix() {
    for (token, kind, rev) in [
        ("<|--", ClassRelationKind::Inheritance, true),
        ("--|>", ClassRelationKind::Inheritance, false),
        ("*--", ClassRelationKind::Composition, true),
        ("--o", ClassRelationKind::Aggregation, false),
        ("..>", ClassRelationKind::Dependency, false),
        ("<--", ClassRelationKind::Association, true),
        ("..", ClassRelationKind::Link, false),
    ] {
        let Diagram::Class(c) = parse_diagram(&format!("classDiagram; A {token} B")).unwrap()
        else {
            panic!()
        };
        assert_eq!((c.relations[0].kind, c.relations[0].reversed), (kind, rev));
    }
}
#[test]
fn state_alias_description_pseudostates_and_cycles() {
    let Diagram::State(s) =
        parse_diagram(include_str!("../../../tests/fixtures/state.mmd")).unwrap()
    else {
        panic!()
    };
    assert_eq!(s.states.len(), 6);
    assert_eq!(s.transitions.len(), 6);
    assert!(s.states.iter().any(|s| s.kind == StateKind::Initial));
    assert!(s.states.iter().any(|s| s.kind == StateKind::Final));
    assert!(s.states.iter().any(|s| s.kind == StateKind::Choice));
    let p = s.states.iter().find(|s| s.id == "Pending").unwrap();
    assert_eq!(p.label, "Waiting for payment");
    assert_eq!(p.description, "Order received");
}
#[test]
fn er_cardinality_keys_and_nonidentifying() {
    let Diagram::Er(e) = parse_diagram(include_str!("../../../tests/fixtures/er.mmd")).unwrap()
    else {
        panic!()
    };
    assert_eq!(e.entities.len(), 4);
    assert_eq!(e.relationships.len(), 3);
    assert_eq!(
        e.relationships[0].source_cardinality,
        Cardinality::ExactlyOne
    );
    assert_eq!(
        e.relationships[0].target_cardinality,
        Cardinality::ZeroOrMore
    );
    assert!(!e.relationships[2].identifying);
    let li = e.entities.iter().find(|e| e.id == "LINE_ITEM").unwrap();
    assert_eq!(li.attributes[0].keys, [AttributeKey::PK, AttributeKey::FK]);
}
#[test]
fn malformed_and_unsupported_family_statements() {
    for source in [
        "classDiagram; class A {; +x",
        "classDiagram; namespace X {",
        "classDiagram; A <?? B",
        "classDiagram; class A; click A x",
        "stateDiagram-v2; state A {",
        "stateDiagram-v2; state A <<unknown>>",
        "stateDiagram-v2; A==>B",
        "erDiagram; A ||--o{ B",
        "erDiagram; A {; int x WRONG; }",
        "erDiagram; A {; int x; int x; }",
        "erDiagram; A oo--o{ B: x",
    ] {
        assert!(parse_diagram(source).is_err(), "accepted {source}");
    }
}
#[test]
fn family_ir_roundtrip_and_invalid_endpoints() {
    for source in [
        include_str!("../../../tests/fixtures/class.mmd"),
        include_str!("../../../tests/fixtures/state.mmd"),
        include_str!("../../../tests/fixtures/er.mmd"),
    ] {
        let d = parse_diagram(source).unwrap();
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(d, serde_json::from_str::<Diagram>(&json).unwrap());
        let mut d = d;
        match &mut d {
            Diagram::Class(c) => c.relations[0].target = 999,
            Diagram::State(s) => s.transitions[0].target = 999,
            Diagram::Er(e) => e.relationships[0].target = 999,
            _ => unreachable!(),
        };
        assert!(d.validate().is_err());
    }
}
