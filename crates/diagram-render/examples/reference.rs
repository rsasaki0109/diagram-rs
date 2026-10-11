//! Native parser output for the development-only Mermaid reference check.
use std::{collections::BTreeMap, fs, path::Path};
fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut cases = BTreeMap::new();
    for name in [
        "branch", "cycle", "groups", "styles", "escaping", "chain", "hero", "extended", "japanese",
        "nested", "parallel",
    ] {
        let path = if name == "hero" {
            root.join("examples/hero.mmd")
        } else {
            root.join(format!("tests/fixtures/{name}.mmd"))
        };
        let source = fs::read_to_string(path).unwrap();
        let graph = diagram_parser::parse(&source).unwrap();
        cases.insert(
            name.to_owned(),
            (source, serde_json::to_value(graph).unwrap()),
        );
    }
    for (i, token) in [
        "-->",
        "--->",
        "----",
        "<-->",
        "<---->",
        "-..->",
        "-...-",
        "<-.->",
        "<-...->",
        "===>",
        "====",
        "<==>",
        "<====>",
        "----------->",
        "<------------>",
        "=============",
        "-...........->",
    ]
    .iter()
    .enumerate()
    {
        let source = format!("flowchart LR; A & B {token}|payload| C & D --> E");
        let graph = diagram_parser::parse(&source).unwrap();
        cases.insert(
            format!("connector-{i}"),
            (source, serde_json::to_value(graph).unwrap()),
        );
    }
    let source = fs::read_to_string(root.join("tests/fixtures/sequence.mmd")).unwrap();
    let diagram_core::Diagram::Sequence(seq) = diagram_parser::parse_diagram(&source).unwrap()
    else {
        unreachable!()
    };
    cases.insert(
        "sequence".to_owned(),
        (
            source,
            serde_json::json!({"family":"sequence", "sequence":seq}),
        ),
    );
    for (name, family) in [
        ("class", "class"),
        ("state", "state"),
        ("er", "er"),
        ("gantt", "gantt"),
    ] {
        let source = fs::read_to_string(root.join(format!("tests/fixtures/{name}.mmd"))).unwrap();
        let diagram = diagram_parser::parse_diagram(&source).unwrap();
        let inner = serde_json::to_value(&diagram)
            .unwrap()
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap()
            .clone();
        cases.insert(
            name.to_owned(),
            (source, serde_json::json!({"family":family,"diagram":inner})),
        );
    }
    for direction in ["TB", "BT", "LR", "RL"] {
        for (i, shape) in [
            "[Node]", "(Node)", "{Node}", "((Node))", "([Node])", "[[Node]]", "[(Node)]",
        ]
        .iter()
        .enumerate()
        {
            let source = format!(
                "flowchart {direction}; A{shape}-->|yes|B{shape}; B-.->C{shape}; C-->A; B==>D{shape}"
            );
            add_case(&mut cases, format!("flow-{direction}-{i}"), source);
        }
    }
    for (i, arrow) in ["->>", "-->>", "->", "-->", "-x", "--x"].iter().enumerate() {
        for (j, note) in [
            "Note left of A: left",
            "Note right of B: right",
            "Note over A,B: both",
        ]
        .iter()
        .enumerate()
        {
            add_case(
                &mut cases,
                format!("sequence-{i}-{j}"),
                format!(
                    "sequenceDiagram; actor A as Client; participant B as Server; A{arrow}B: message; B{arrow}B: self; {note}"
                ),
            );
        }
    }
    for (i, arrow) in [
        "<|--", "--|>", "<|..", "..|>", "*--", "--*", "o--", "--o", "<--", "-->", "<..", "..>",
        "--", "..",
    ]
    .iter()
    .enumerate()
    {
        add_case(
            &mut cases,
            format!("class-{i}"),
            format!(
                "classDiagram; class A {{; +String id; +save() bool; }}; class B; A \"1\" {arrow} \"*\" B: related"
            ),
        );
    }
    for (i, left) in ["||", "|o", "}|", "}o"].iter().enumerate() {
        for (j, right) in ["||", "o|", "|{", "o{"].iter().enumerate() {
            for identifying in [true, false] {
                add_case(
                    &mut cases,
                    format!("er-{i}-{j}-{identifying}"),
                    format!(
                        "erDiagram; A {left}{}{right} B: related; A {{; int id PK; string name \"comment\"; }}",
                        if identifying { "--" } else { ".." }
                    ),
                );
            }
        }
    }
    for (i, kind) in ["choice", "fork", "join"].iter().enumerate() {
        for direction in ["TB", "LR", "RL", "BT"] {
            add_case(
                &mut cases,
                format!("state-{i}-{direction}"),
                format!(
                    "stateDiagram-v2; direction {direction}; state S <<{kind}>>; [*]-->A; A-->S: ready; S-->B: ok; B-->[*]; B-->A: retry"
                ),
            );
        }
    }
    for (i, start) in ["2024-02-28", "2024-12-31", "1900-02-28", "2000-02-28"]
        .iter()
        .enumerate()
    {
        for duration in ["1d", "3d", "1w"] {
            add_case(
                &mut cases,
                format!("gantt-{i}-{duration}"),
                format!(
                    "gantt; dateFormat YYYY-MM-DD; todayMarker off; First: first, {start}, {duration}; Next: next, after first, 2d; Finish: milestone, finish, after next, 0d"
                ),
            );
        }
    }
    println!("{}", serde_json::to_string(&cases).unwrap());
}

fn add_case(
    cases: &mut BTreeMap<String, (String, serde_json::Value)>,
    name: String,
    source: String,
) {
    // Family grammars upstream use newlines (class bodies reject semicolons).
    let source = source.replace(';', "\n");
    let diagram = diagram_parser::parse_diagram(&source).unwrap();
    let value = match diagram {
        diagram_core::Diagram::Flowchart(g) => serde_json::to_value(g).unwrap(),
        diagram_core::Diagram::Sequence(s) => serde_json::json!({"family":"sequence","sequence":s}),
        other => {
            let family = other.kind().to_lowercase();
            let value = serde_json::to_value(&other).unwrap();
            let inner = value.as_object().unwrap().values().next().unwrap();
            serde_json::json!({"family":family,"diagram":inner})
        }
    };
    cases.insert(name, (source, value));
}
