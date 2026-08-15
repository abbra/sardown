use krilla::text::{GlyphId, KrillaGlyph};
use md2pdf_layout::PositionedGlyph;

/// krilla's `Glyph` trait expects advances/offsets normalized by the font's units-per-em,
/// not raw font units — divide before constructing `KrillaGlyph` (see krilla::text::Glyph docs).
pub fn to_krilla_glyph(
    glyph: &PositionedGlyph,
    text_range: std::ops::Range<usize>,
    units_per_em: f32,
) -> KrillaGlyph {
    KrillaGlyph::new(
        GlyphId::new(glyph.glyph_id as u32),
        glyph.x_advance / units_per_em,
        0.0, // x_offset: Phase 1 glyphs carry no per-glyph offset (kerning already baked into x_advance)
        0.0, // y_offset
        0.0, // y_advance: horizontal text only in Phase 1
        text_range,
        None, // location: not needed until variable-font support, out of scope
    )
}
