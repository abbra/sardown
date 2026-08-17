use krilla::text::{GlyphId, KrillaGlyph};
use sardown_layout::PositionedGlyph;

/// krilla's `Glyph` trait expects advances/offsets normalized by units-per-em (i.e. a fraction
/// of one em, which krilla then scales by the `font_size` passed to `draw_glyphs`). cosmic-text's
/// `LayoutGlyph::w` ("width of hitbox") is already in pixels at the font size used for shaping —
/// dividing by the font's units-per-em (typically 1000-2048) instead of by that same shaping
/// `size` shrinks every advance by roughly two orders of magnitude, since krilla then multiplies
/// by `font_size` again. Confirmed visually: dividing by units-per-em produced PDFs where nearly
/// all glyphs overlapped at (or near) the same position instead of flowing across the line.
pub fn to_krilla_glyph(glyph: &PositionedGlyph, size: f32) -> KrillaGlyph {
    KrillaGlyph::new(
        GlyphId::new(glyph.glyph_id as u32),
        glyph.x_advance / size,
        0.0, // x_offset: no per-glyph offset tracked (kerning already baked into x_advance)
        0.0, // y_offset
        0.0, // y_advance: horizontal text only
        glyph.cluster.clone(),
        None, // location: not needed until variable-font support, out of scope
    )
}
