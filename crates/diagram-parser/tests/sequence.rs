use diagram_core::{sequence::*, *};
use diagram_parser::parse_diagram;
fn seq(s: &str) -> Sequence {
    let Diagram::Sequence(s) = parse_diagram(s).unwrap() else {
        panic!()
    };
    s
}
#[test]
fn participants_messages_aliases_and_arrows() {
    let s = seq(
        "sequenceDiagram; actor A as Client; participant B as Server; A->>B: send; B-->>A: reply; A->B: open; B-->A: dotted open; A-xB: lost; B--xA: dotted lost",
    );
    assert_eq!(s.participants[0].kind, ParticipantKind::Actor);
    assert_eq!(s.participants[1].label, "Server");
    let kinds: Vec<_> = s
        .events
        .iter()
        .filter_map(|e| {
            if let SequenceEvent::Message { kind, .. } = e {
                Some(*kind)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        kinds,
        [
            MessageKind::SolidArrow,
            MessageKind::DottedArrow,
            MessageKind::SolidOpen,
            MessageKind::DottedOpen,
            MessageKind::SolidCross,
            MessageKind::DottedCross
        ]
    );
}
#[test]
fn fixture_notes_scopes_and_activations() {
    let s = seq(include_str!("../../../tests/fixtures/sequence.mmd"));
    assert!(s.autonumber);
    assert_eq!(s.participants.len(), 3);
    assert_eq!(
        s.events
            .iter()
            .filter(|e| matches!(e, SequenceEvent::Message { .. }))
            .count(),
        7
    );
    for text in [
        "sequenceDiagram; A->>+B: start; B-->>-A: finish",
        "sequenceDiagram; par work; A->>B: a; and other; A->>B: b; end",
        "sequenceDiagram; critical write; A->>B: work; option failure; B-->>A: fail; end",
    ] {
        seq(text);
    }
}
#[test]
fn unsupported_and_unbalanced_sequence_input() {
    for text in [
        "A->>B: hi; end",
        "deactivate A",
        "activate A",
        "alt ok; A->>B: yes",
        "loop retry; else fail; end",
        "Note left of A,B: bad",
        "A<<->>B: bad",
        "A->>B: ok; destroy B",
        "participant A as x; rect red; end",
        "autonumber 10 2; A->>B: hi",
    ] {
        assert!(
            parse_diagram(&format!("sequenceDiagram; {text}")).is_err(),
            "accepted {text}"
        );
    }
    let e = parse_diagram("sequenceDiagram\nA->>B: café\nwrong").unwrap_err();
    assert_eq!(e.span.line, 3);
    assert_eq!(e.span.column, 1);
}
#[test]
fn invalid_ir_and_sequence_limits() {
    let mut s = seq("sequenceDiagram; A->>B: hi");
    if let SequenceEvent::Message { target, .. } = &mut s.events[0] {
        *target = 99;
    };
    assert!(s.validate().is_err());
    let mut source = "sequenceDiagram\n".to_owned();
    for i in 0..513 {
        source.push_str(&format!("participant P{i}\n"));
    }
    assert!(parse_diagram(&source).is_err());
    let s = format!("sequenceDiagram; A->>B: hi; {}", "loop x; ".repeat(33));
    assert!(parse_diagram(&s).is_err());
    let s = format!(
        "sequenceDiagram; participant A; {}",
        "Note over A: hi; ".repeat(2049)
    );
    assert!(parse_diagram(&s).is_err());
}
