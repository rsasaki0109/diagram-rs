# Bundled fonts

- DejaVu Sans, unmodified: `DejaVuSans.ttf`, notice in `LICENSE.txt`.
- Noto Sans JP variable font, unmodified: `NotoSansJP.ttf`, SIL Open Font License
  1.1 in `NotoSansJP-OFL.txt`. Rendering selects weight 400 without modifying the
  font binary. Downloaded from Google Fonts `ofl/notosansjp/NotoSansJP[wght].ttf`.
  SHA-256: `c2f3b4d463500a2ddcd3849cded1fceeb9fd6d1c32e6cbecd568453ba50fc68f`.

The default `cjk` feature uses Noto Sans JP when a character is unavailable in
DejaVu Sans. Shaping, measurement and emitted outlines select the same font.
Latin-only SVG output remains unchanged. SVGs using fallback glyphs include the
Noto license notice as well as DejaVu's. Mixed-script paragraph bidi and emoji
fallback are not fully supported.

The Japanese font is about 9.6 MB before compression and increases native/WASM
sizes. Build the CLI or WASM crate with `--no-default-features` for a smaller
Latin-only default font, or supply a custom font using the existing API. A custom
font replaces the default collection; it does not inherit bundled fallback fonts.
Ship both notices when redistributing a standard build, and preserve custom-font
licenses separately.
