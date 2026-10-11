//! Browser bindings. No DOM access, Mermaid.js, or JavaScript layout dependencies.
use wasm_bindgen::prelude::*;
/// Render Mermaid diagram source to a self-contained SVG string.
#[wasm_bindgen]
pub fn render(source: &str) -> Result<String, JsValue> {
    let graph =
        diagram_parser::parse_diagram(source).map_err(|e| JsValue::from_str(&e.to_string()))?;
    diagram_render::render_diagram(&graph).map_err(|e| JsValue::from_str(&e.to_string()))
}
/// Render with caller-provided TrueType/OpenType bytes for extra font coverage.
#[wasm_bindgen(js_name = renderWithFont)]
pub fn render_with_font(source: &str, bytes: &[u8]) -> Result<String, JsValue> {
    let graph =
        diagram_parser::parse_diagram(source).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let font = diagram_render::Font::from_bytes(bytes).map_err(|e| JsValue::from_str(&e))?;
    diagram_render::render_diagram_with_font(&graph, &font)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
/// Validate syntax and IR invariants without rendering.
#[wasm_bindgen]
pub fn check(source: &str) -> Result<(), JsValue> {
    diagram_parser::parse_diagram(source)
        .map(|_| ())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
