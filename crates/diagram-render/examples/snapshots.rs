//! Explicitly regenerate reviewed SVG/PNG regression baselines.
use std::{fs, path::Path};
fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in [
        "branch", "cycle", "groups", "styles", "escaping", "chain", "hero", "extended", "sequence",
        "class", "state", "er", "gantt", "japanese", "nested", "parallel",
    ] {
        let input = if name == "hero" {
            root.join("examples/hero.mmd")
        } else {
            root.join(format!("tests/fixtures/{name}.mmd"))
        };
        let g = diagram_parser::parse_diagram(&fs::read_to_string(input).unwrap()).unwrap();
        let svg = diagram_render::render_diagram(&g).unwrap();
        fs::write(root.join(format!("tests/snapshots/{name}.svg")), &svg).unwrap();
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
        let size = tree.size().to_int_size();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );
        pixmap
            .save_png(root.join(format!("tests/snapshots/{name}.png")))
            .unwrap();
        if name == "hero" {
            fs::write(root.join("assets/hero.svg"), svg).unwrap();
        }
    }
}
