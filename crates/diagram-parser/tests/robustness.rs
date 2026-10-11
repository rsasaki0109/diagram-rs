//! Deterministic mutation campaigns catch parser panics across all family scanners.
use diagram_parser::parse_diagram;
#[test]
fn mutated_family_sources_never_panic() {
    let sources = [
        include_str!("../../../tests/fixtures/nested.mmd"),
        include_str!("../../../tests/fixtures/sequence.mmd"),
        include_str!("../../../tests/fixtures/class.mmd"),
        include_str!("../../../tests/fixtures/state.mmd"),
        include_str!("../../../tests/fixtures/er.mmd"),
        include_str!("../../../tests/fixtures/gantt.mmd"),
    ];
    let alphabet: Vec<_> = ";:\n\r\"[]{}()<>-=.|+#\\é漢\0".chars().collect();
    let mut seed = 41_u64;
    for source in sources {
        for _ in 0..600 {
            let mut text: Vec<_> = source.chars().collect();
            for _ in 0..4 {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let at = (seed >> 32) as usize % (text.len() + 1);
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                match seed % 3 {
                    0 => {
                        text.insert(at, alphabet[(seed >> 32) as usize % alphabet.len()]);
                    }
                    1 if at < text.len() => {
                        text.remove(at);
                    }
                    2 if at < text.len() => {
                        text[at] = alphabet[(seed >> 32) as usize % alphabet.len()]
                    }
                    _ => {}
                }
            }
            let text: String = text.into_iter().collect();
            let result = std::panic::catch_unwind(|| parse_diagram(&text));
            assert!(result.is_ok(), "parser panic for {text:?}");
            if let Ok(d) = result.unwrap() {
                d.validate().unwrap();
            }
        }
    }
}
#[test]
fn arbitrary_unicode_and_control_input_never_panics() {
    let alphabet: Vec<_> = "ABCxyz123 \n\r\t\0é漢😀[]{};:\"->|\\%".chars().collect();
    let mut seed = 29_u64;
    for header in [
        "flowchart TB",
        "sequenceDiagram",
        "classDiagram",
        "stateDiagram-v2",
        "erDiagram",
        "gantt",
    ] {
        for _ in 0..400 {
            let mut text = format!("{header}\n");
            for _ in 0..128 {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                text.push(alphabet[(seed >> 32) as usize % alphabet.len()]);
            }
            assert!(
                std::panic::catch_unwind(|| parse_diagram(&text)).is_ok(),
                "{text:?}"
            );
        }
    }
}
