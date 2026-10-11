//! Native SVG rendering with shaped glyph outlines, so geometry and typography
//! use the same bundled font on desktop, WASM, and machines without fonts.
use diagram_core::*;
use diagram_layout::{Layout, layout};
use rustybuzz::{
    Face, UnicodeBuffer,
    ttf_parser::{GlyphId, OutlineBuilder},
};
use std::{collections::BTreeSet, fmt::Write};

const FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/DejaVuSans.ttf");
#[cfg(feature = "cjk")]
const CJK_BYTES: &[u8] = include_bytes!("../../../assets/fonts/NotoSansJP.ttf");
#[cfg(feature = "cjk")]
const CJK_LICENSE: &str = include_str!("../../../assets/fonts/NotoSansJP-OFL.txt");
/// A shaping and outline font. Supply another font to cover additional scripts.
pub struct Font<'a> {
    face: Face<'a>,
    fallback: Vec<Face<'a>>,
    license: Option<&'static str>,
}
struct Run {
    infos: Vec<u32>,
    positions: Vec<rustybuzz::GlyphPosition>,
    min_x: f64,
    width: f64,
    ascender: f64,
    descender: f64,
    line_gap: f64,
}
impl Default for Font<'static> {
    fn default() -> Self {
        let font = Self::from_bytes(FONT_BYTES).expect("bundled DejaVu Sans is valid");
        #[cfg(feature = "cjk")]
        let mut font = font;
        #[cfg(feature = "cjk")]
        {
            let mut cjk = Face::from_slice(CJK_BYTES, 0).expect("bundled Noto Sans JP is valid");
            cjk.set_variations(&[rustybuzz::Variation {
                tag: rustybuzz::ttf_parser::Tag::from_bytes(b"wght"),
                value: 400.0,
            }]);
            font.fallback.push(cjk);
        }
        font
    }
}
impl<'a> Font<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, String> {
        Ok(Self {
            face: Face::from_slice(bytes, 0).ok_or("invalid TrueType/OpenType font")?,
            fallback: Vec::new(),
            license: (bytes == FONT_BYTES)
                .then_some(include_str!("../../../assets/fonts/LICENSE.txt")),
        })
    }
    fn scale(&self, size: f64) -> f64 {
        size / self.face.units_per_em() as f64
    }
    fn face_at(&self, index: usize) -> &Face<'a> {
        if index == 0 {
            &self.face
        } else {
            &self.fallback[index - 1]
        }
    }
    fn line_height(&self, run: &Run, size: f64) -> f64 {
        ((run.ascender - run.descender + run.line_gap) * self.scale(size)).max(size * 1.2)
    }
    fn run(&self, line: &str) -> Result<Run, String> {
        let text = line.replace('\t', "    ");
        let mut segments: Vec<(usize, String)> = Vec::new();
        for ch in text.chars() {
            let index = if self.face.glyph_index(ch).is_some() {
                0
            } else {
                self.fallback.iter().position(|f|f.glyph_index(ch).is_some()).map(|i|i+1).ok_or("font does not cover a label character; supply a font with the required glyphs")?
            };
            if let Some((_, s)) = segments.last_mut().filter(|(i, _)| *i == index) {
                s.push(ch);
            } else {
                segments.push((index, ch.to_string()));
            }
        }
        let mut run = Run {
            infos: Vec::new(),
            positions: Vec::new(),
            min_x: 0.0,
            width: 0.0,
            ascender: self.face.ascender() as f64,
            descender: self.face.descender() as f64,
            line_gap: self.face.line_gap() as f64,
        };
        let mut cursor = 0.0_f64;
        let mut max_x = 0.0_f64;
        for (index, text) in segments {
            let face = self.face_at(index);
            let factor = self.face.units_per_em() as f64 / face.units_per_em() as f64;
            run.ascender = run.ascender.max(face.ascender() as f64 * factor);
            run.descender = run.descender.min(face.descender() as f64 * factor);
            run.line_gap = run.line_gap.max(face.line_gap() as f64 * factor);
            let mut b = UnicodeBuffer::new();
            b.push_str(&text);
            b.guess_segment_properties();
            let shaped = rustybuzz::shape(face, &[], b);
            for (g, p) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                if g.glyph_id == 0 {
                    return Err("font does not cover a label character; supply a font with the required glyphs".into());
                }
                let mut p = *p;
                p.x_advance = (p.x_advance as f64 * factor).round() as i32;
                p.y_advance = (p.y_advance as f64 * factor).round() as i32;
                p.x_offset = (p.x_offset as f64 * factor).round() as i32;
                p.y_offset = (p.y_offset as f64 * factor).round() as i32;
                if let Some(b) = face.glyph_bounding_box(GlyphId(g.glyph_id as u16)) {
                    run.min_x = run
                        .min_x
                        .min(cursor + p.x_offset as f64 + b.x_min as f64 * factor);
                    max_x = max_x.max(cursor + p.x_offset as f64 + b.x_max as f64 * factor);
                }
                cursor += p.x_advance as f64;
                run.min_x = run.min_x.min(cursor);
                max_x = max_x.max(cursor);
                run.infos.push(index as u32 * 65536 + g.glyph_id);
                run.positions.push(p);
            }
        }
        run.width = max_x - run.min_x;
        Ok(run)
    }
    fn outline(&self, id: u32) -> (String, f64) {
        let face = self.face_at((id / 65536) as usize);
        let mut path = PathBuilder(String::new());
        face.outline_glyph(GlyphId((id % 65536) as u16), &mut path);
        (
            path.0,
            self.face.units_per_em() as f64 / face.units_per_em() as f64,
        )
    }
    fn glyphs(&self, text: &str) -> Result<BTreeSet<u32>, String> {
        let mut glyphs = BTreeSet::new();
        for line in text.split('\n') {
            glyphs.extend(self.run(line)?.infos);
        }
        Ok(glyphs)
    }
    fn text(
        &self,
        out: &mut String,
        text: &str,
        center: Point,
        size: f64,
        color: &str,
    ) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }
        writeln!(
            out,
            "<g fill=\"{}\" aria-label=\"{}\"><title>{}</title>",
            escape(color),
            escape(text),
            escape(text)
        )
        .unwrap();
        let lines: Vec<_> = text.split('\n').collect();
        let runs = lines
            .iter()
            .map(|line| self.run(line))
            .collect::<Result<Vec<_>, _>>()?;
        let scale = self.scale(size);
        let height = runs.iter().map(|r| self.line_height(r, size)).sum::<f64>();
        let mut y = center.y - height / 2.0;
        for run in runs {
            let lh = self.line_height(&run, size);
            let baseline =
                y + (lh - (run.ascender - run.descender) * scale) / 2.0 + run.ascender * scale;
            y += lh;
            let mut cursor = center.x - run.width * scale / 2.0 - run.min_x * scale;
            for (&id, p) in run.infos.iter().zip(&run.positions) {
                write!(out,"<use href=\"#glyph-{id}\" transform=\"translate({:.3} {:.3}) scale({:.6} {:.6})\"/>",cursor+p.x_offset as f64*scale,baseline-p.y_offset as f64*scale,scale,-scale).unwrap();
                cursor += p.x_advance as f64 * scale;
            }
        }
        out.push_str("</g>\n");
        Ok(())
    }
}
impl TextMeasure for Font<'_> {
    fn measure(&self, text: &str, size: f64) -> Result<Size, String> {
        if !size.is_finite() || !(1.0..=48.0).contains(&size) {
            return Err("font size must be between 1 and 48".into());
        }
        let mut width = 0.0_f64;
        let mut height = 0.0;
        for line in text.split('\n') {
            let r = self.run(line)?;
            width = width.max(r.width * self.scale(size));
            height += self.line_height(&r, size);
        }
        Ok(Size { width, height })
    }
}
/// Render using the bundled font and the graph's configuration.
///
/// ```
/// let graph = diagram_parser::parse("flowchart LR; A[Input] --> B[SVG]")?;
/// let svg = diagram_render::render(&graph)?;
/// assert!(svg.starts_with("<svg"));
/// # Ok::<(), diagram_core::Diagnostic>(())
/// ```
pub fn render(graph: &Graph) -> Result<String, Diagnostic> {
    render_with_font(graph, &Font::default())
}
/// Render with a caller-supplied font, retaining native shaping and measurement.
pub fn render_with_font(graph: &Graph, font: &Font<'_>) -> Result<String, Diagnostic> {
    let geometry = layout(graph, font)?;
    svg(graph, &geometry, font)
}
/// Render precomputed geometry. Use geometry produced by `diagram_layout::layout`.
pub fn svg(graph: &Graph, geometry: &Layout, font: &Font<'_>) -> Result<String, Diagnostic> {
    graph.validate()?;
    let fail = |s| {
        Diagnostic::new(
            s,
            Span {
                line: 1,
                column: 1,
                ..Span::default()
            },
        )
    };
    if geometry.nodes.len() != graph.nodes.len()
        || geometry.edges.len() != graph.edges.len()
        || geometry.groups.len() != graph.groups.len()
    {
        return Err(fail("layout does not match graph".into()));
    }
    let (bg, fill, stroke, text, group_fill) = match graph.config.theme {
        Theme::Light => ("#f8fafc", "#ffffff", "#475569", "#0f172a", "#eef2ff"),
        Theme::Dark => ("#0f172a", "#1e293b", "#94a3b8", "#f1f5f9", "#172033"),
    };
    let mut out = String::new();
    writeln!(out,"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.3}\" height=\"{:.3}\" viewBox=\"0 0 {:.3} {:.3}\" role=\"img\" aria-label=\"Flowchart\">",geometry.size.width,geometry.size.height,geometry.size.width,geometry.size.height).unwrap();
    out.push_str("<title>Flowchart rendered by diagram-rs</title>\n");
    if let Some(license) = font.license {
        writeln!(out, "<metadata>{}</metadata>", escape(license)).unwrap();
    }
    let mut glyphs = BTreeSet::new();
    for label in graph
        .nodes
        .iter()
        .map(|n| n.label.as_str())
        .chain(graph.edges.iter().map(|e| e.label.as_str()))
        .chain(graph.groups.iter().map(|g| g.label.as_str()))
    {
        glyphs.extend(font.glyphs(label).map_err(fail)?);
    }
    #[cfg(feature = "cjk")]
    if glyphs.iter().any(|id| *id >= 65536) {
        writeln!(out, "<metadata>{}</metadata>", escape(CJK_LICENSE)).unwrap();
    }
    out.push_str("<defs>\n");
    for id in glyphs {
        let (path, factor) = font.outline(id);
        if factor == 1.0 {
            writeln!(out, "<path id=\"glyph-{id}\" d=\"{path}\"/>").unwrap();
        } else {
            writeln!(
                out,
                "<path id=\"glyph-{id}\" transform=\"scale({factor:.6})\" d=\"{path}\"/>"
            )
            .unwrap();
        }
    }
    for (i, e) in graph
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.directed || e.source_arrow)
    {
        let orient = if e.source_arrow {
            "auto-start-reverse"
        } else {
            "auto"
        };
        writeln!(out,"<marker id=\"arrow-{i}\" markerUnits=\"userSpaceOnUse\" markerWidth=\"10\" markerHeight=\"10\" refX=\"10\" refY=\"5\" orient=\"{orient}\"><path d=\"M0 0 L10 5 L0 10 Z\" fill=\"{}\"/></marker>",escape(e.style.stroke.as_deref().unwrap_or(stroke))).unwrap();
    }
    out.push_str("</defs>\n");
    writeln!(out, "<rect width=\"100%\" height=\"100%\" fill=\"{bg}\"/>").unwrap();
    for (i, g) in graph.groups.iter().enumerate() {
        let r = geometry.groups[i];
        writeln!(out,"<g id=\"group-{i}\"><rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" rx=\"10\" fill=\"{group_fill}\" stroke=\"{stroke}\" stroke-dasharray=\"5 4\"/>",r.x,r.y,r.width,r.height).unwrap();
        font.text(
            &mut out,
            &g.label,
            Point {
                x: r.center().x,
                y: r.y + 17.0,
            },
            14.0,
            text,
        )
        .map_err(fail)?;
        out.push_str("</g>\n");
    }
    for (i, e) in graph.edges.iter().enumerate() {
        let route = &geometry.edges[i];
        let color = e.style.stroke.as_deref().unwrap_or(stroke);
        let width = e
            .style
            .stroke_width
            .unwrap_or(if e.kind == LineKind::Thick { 3.0 } else { 1.5 });
        write!(out,"<path id=\"edge-{i}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{width:.3}\" stroke-linejoin=\"round\"",escape(color)).unwrap();
        if e.kind == LineKind::Dotted {
            out.push_str(" stroke-dasharray=\"4 4\"");
        }
        if e.directed {
            write!(out, " marker-end=\"url(#arrow-{i})\"").unwrap();
        }
        if e.source_arrow {
            write!(out, " marker-start=\"url(#arrow-{i})\"").unwrap();
        }
        out.push_str(" d=\"");
        for (j, p) in route.points.iter().enumerate() {
            write!(
                out,
                "{} {:.3} {:.3} ",
                if j == 0 { "M" } else { "L" },
                p.x,
                p.y
            )
            .unwrap();
        }
        out.push_str("\"/>\n");
    }
    for (i, n) in graph.nodes.iter().enumerate() {
        let r = geometry.nodes[i];
        let c = r.center();
        writeln!(out,"<g id=\"node-{i}\" data-node-id=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.3}\">",escape(&n.id),escape(n.style.fill.as_deref().unwrap_or(fill)),escape(n.style.stroke.as_deref().unwrap_or(stroke)),n.style.stroke_width.unwrap_or(1.5)).unwrap();
        match n.shape {
            Shape::Rectangle | Shape::Rounded | Shape::Bar => {
                writeln!(
                    out,
                    "<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" rx=\"{}\"/>",
                    r.x,
                    r.y,
                    r.width,
                    r.height,
                    if n.shape == Shape::Rounded { 10 } else { 0 }
                )
                .unwrap();
            }
            Shape::Circle | Shape::Initial | Shape::Final => {
                writeln!(
                    out,
                    "<ellipse cx=\"{:.3}\" cy=\"{:.3}\" rx=\"{:.3}\" ry=\"{:.3}\"/>",
                    c.x,
                    c.y,
                    r.width / 2.0,
                    r.height / 2.0
                )
                .unwrap();
            }
            Shape::Stadium => {
                writeln!(
                    out,
                    "<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" rx=\"{:.3}\"/>",
                    r.x,
                    r.y,
                    r.width,
                    r.height,
                    r.height / 2.0
                )
                .unwrap();
            }
            Shape::Subroutine => {
                writeln!(out, "<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\"/><path fill=\"none\" d=\"M {:.3} {:.3} V {:.3} M {:.3} {:.3} V {:.3}\"/>", r.x, r.y, r.width, r.height, r.x + 10.0, r.y, r.y + r.height, r.x + r.width - 10.0, r.y, r.y + r.height).unwrap();
            }
            Shape::Cylinder => {
                let cap = 12.0;
                let rx = r.width / 2.0;
                let top = r.y + cap;
                let bottom = r.y + r.height - cap;
                let right = r.x + r.width;
                writeln!(out, "<path d=\"M {:.3} {top:.3} A {rx:.3} {cap} 0 0 1 {right:.3} {top:.3} V {bottom:.3} A {rx:.3} {cap} 0 0 1 {:.3} {bottom:.3} Z\"/><path fill=\"none\" d=\"M {:.3} {top:.3} A {rx:.3} {cap} 0 0 0 {right:.3} {top:.3}\"/>", r.x, r.x, r.x).unwrap();
            }
            Shape::Diamond => {
                writeln!(
                    out,
                    "<path d=\"M {:.3} {:.3} L {:.3} {:.3} L {:.3} {:.3} L {:.3} {:.3} Z\"/>",
                    c.x,
                    r.y,
                    r.x + r.width,
                    c.y,
                    c.x,
                    r.y + r.height,
                    r.x,
                    c.y
                )
                .unwrap();
            }
        }
        // Text gets its own stroke-free group; glyphs must not inherit node stroke.
        out.push_str("<g stroke=\"none\">");
        font.text(
            &mut out,
            &n.label,
            c,
            n.style.font_size.unwrap_or(15.0),
            n.style.color.as_deref().unwrap_or(text),
        )
        .map_err(fail)?;
        out.push_str("</g></g>\n");
    }
    for (i, e) in graph
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.label.is_empty())
    {
        let r = geometry.edges[i].label;
        writeln!(
            out,
            "<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" rx=\"4\" fill=\"{}\"/>",
            r.x,
            r.y,
            r.width,
            r.height,
            escape(e.style.fill.as_deref().unwrap_or(bg))
        )
        .unwrap();
        font.text(
            &mut out,
            &e.label,
            r.center(),
            e.style.font_size.unwrap_or(13.0),
            e.style.color.as_deref().unwrap_or(text),
        )
        .map_err(fail)?;
    }
    out.push_str("</svg>\n");
    Ok(namespace(out))
}
fn namespace(mut out: String) -> String {
    // Namespace generated fragment IDs by the complete output. Different diagrams
    // embedded in one HTML page must not borrow each other's marker/glyph defs.
    // Only generated attribute prefixes are rewritten; escaped input cannot match.
    let fingerprint = out.bytes().fold(0xcbf29ce484222325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    let namespace = format!("{fingerprint:016x}-");
    for kind in [
        "glyph",
        "arrow",
        "node",
        "edge",
        "group",
        "participant",
        "message",
        "activation",
        "fragment",
        "note",
        "task",
    ] {
        out = out.replace(
            &format!("id=\"{kind}-"),
            &format!("id=\"{kind}-{namespace}"),
        );
    }
    out = out.replace("href=\"#glyph-", &format!("href=\"#glyph-{namespace}"));
    out = out.replace(
        "marker-end=\"url(#arrow-",
        &format!("marker-end=\"url(#arrow-{namespace}"),
    );
    out = out.replace(
        "marker-start=\"url(#arrow-",
        &format!("marker-start=\"url(#arrow-{namespace}"),
    );
    out
}
/// XML escaping for text and attributes, including quote characters.
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
struct PathBuilder(String);
impl OutlineBuilder for PathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        write!(self.0, "M{x:.1} {y:.1} ").unwrap();
    }
    fn line_to(&mut self, x: f32, y: f32) {
        write!(self.0, "L{x:.1} {y:.1} ").unwrap();
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        write!(self.0, "Q{x1:.1} {y1:.1} {x:.1} {y:.1} ").unwrap();
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        write!(self.0, "C{x1:.1} {y1:.1} {x2:.1} {y2:.1} {x:.1} {y:.1} ").unwrap();
    }
    fn close(&mut self) {
        self.0.push_str("Z ");
    }
}

mod canvas;
mod class;
mod er;
mod gantt;
mod sequence;
mod state;
/// Render any implemented diagram family using the bundled font.
pub fn render_diagram(diagram: &Diagram) -> Result<String, Diagnostic> {
    render_diagram_with_font(diagram, &Font::default())
}
/// Render any implemented diagram family with caller-supplied font bytes.
pub fn render_diagram_with_font(diagram: &Diagram, font: &Font<'_>) -> Result<String, Diagnostic> {
    match diagram {
        Diagram::Flowchart(g) => render_with_font(g, font),
        Diagram::Sequence(s) => sequence::render_sequence(s, font),
        Diagram::Class(c) => class::render_class(c, font),
        Diagram::State(s) => state::render_state(s, font),
        Diagram::Er(e) => er::render_er(e, font),
        Diagram::Gantt(g) => gantt::render_gantt(g, font),
    }
}
