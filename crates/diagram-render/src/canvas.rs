//! SVG and glyph helpers shared by the specialized diagram renderers.
use super::*;
pub(super) struct Canvas<'a, 'f> {
    pub body: String,
    pub defs: String,
    pub font: &'a Font<'f>,
    pub ink: &'static str,
    pub fill: &'static str,
    pub stroke: &'static str,
    pub background: &'static str,
    glyphs: BTreeSet<u32>,
}
impl<'a, 'f> Canvas<'a, 'f> {
    pub fn new(font: &'a Font<'f>, theme: Theme) -> Self {
        let (background, fill, stroke, ink) = match theme {
            Theme::Light => ("#f8fafc", "#ffffff", "#475569", "#0f172a"),
            Theme::Dark => ("#0f172a", "#1e293b", "#94a3b8", "#f1f5f9"),
        };
        Self {
            body: String::new(),
            defs: String::new(),
            font,
            ink,
            fill,
            stroke,
            background,
            glyphs: BTreeSet::new(),
        }
    }
    pub fn text(&mut self, label: &str, center: Point, size: f64) -> Result<(), Diagnostic> {
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
        self.glyphs.extend(self.font.glyphs(label).map_err(fail)?);
        self.font
            .text(&mut self.body, label, center, size, self.ink)
            .map_err(fail)
    }
    pub fn rect(&mut self, r: Rect, fill: &str, radius: f64) {
        writeln!(self.body,"<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" rx=\"{radius}\" fill=\"{fill}\" stroke=\"{}\"/>",r.x,r.y,r.width,r.height,self.stroke).unwrap();
    }
    pub fn finish(self, size: Size, title: &str) -> String {
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.3}\" height=\"{:.3}\" viewBox=\"0 0 {:.3} {:.3}\" role=\"img\" aria-label=\"{}\"><title>{}</title>\n",
            size.width,
            size.height,
            size.width,
            size.height,
            escape(title),
            escape(title)
        );
        if let Some(license) = self.font.license {
            writeln!(out, "<metadata>{}</metadata>", escape(license)).unwrap();
        }
        out.push_str("<defs>\n");
        #[cfg(feature = "cjk")]
        if self.glyphs.iter().any(|id| *id >= 65536) {
            writeln!(out, "<metadata>{}</metadata>", escape(CJK_LICENSE)).unwrap();
        }
        for id in self.glyphs {
            let (path, factor) = self.font.outline(id);
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
        out.push_str(&self.defs);
        out.push_str("</defs>\n");
        writeln!(
            out,
            "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
            self.background
        )
        .unwrap();
        out.push_str(&self.body);
        out.push_str("</svg>\n");
        namespace(out)
    }
}
