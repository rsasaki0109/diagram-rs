use diagram_parser::parse_diagram;
use diagram_render::render_diagram;
#[test]
fn mutated_valid_ir_sources_never_panic_during_rendering() {
    let sources = [
        include_str!("../../../tests/fixtures/nested.mmd"),
        include_str!("../../../tests/fixtures/sequence.mmd"),
        include_str!("../../../tests/fixtures/class.mmd"),
        include_str!("../../../tests/fixtures/state.mmd"),
        include_str!("../../../tests/fixtures/er.mmd"),
        include_str!("../../../tests/fixtures/gantt.mmd"),
    ];
    let mut seed = 11_u64;
    let mut rendered = 0;
    for source in sources {
        for _ in 0..120 {
            let mut text: Vec<_> = source.chars().collect();
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let at = (seed >> 32) as usize % text.len();
            text[at] = ['A', '1', ' ', '\n', '"', ';', 'é', '\0'][(seed % 8) as usize];
            let text: String = text.into_iter().collect();
            if let Ok(d) = parse_diagram(&text) {
                let result = std::panic::catch_unwind(|| render_diagram(&d));
                assert!(result.is_ok(), "renderer panic for {text:?}");
                if let Ok(svg) = result.unwrap() {
                    roxmltree::Document::parse(&svg).unwrap();
                    rendered += 1;
                }
            }
        }
    }
    assert!(
        rendered > 100,
        "campaign exercised too few valid diagrams: {rendered}"
    );
}
