use md2pdf_style::{SlideLayoutStyle, Stylesheet};

/// Builds a per-slide `Stylesheet` clone in two ordered steps: first `layout`'s own overrides
/// establish this slide's *unscaled* (scale `1.0`) starting values, then every font-size-bearing
/// field is multiplied by `scale` on top of that. `[header]`/`[footer]` font sizes are never
/// touched by either step -- they're per-document chrome, not slide content, and shrinking a
/// slide's body text must not shrink the page-number footer too.
pub fn build_slide_stylesheet(base: &Stylesheet, layout: &SlideLayoutStyle, scale: f32) -> Stylesheet {
    let mut sheet = base.clone();

    if let Some(alignment) = layout.alignment {
        sheet.typography.alignment = alignment;
    }
    if let Some(body_size_pt) = layout.body_size_pt {
        sheet.typography.body_size_pt = body_size_pt;
    }
    if let Some(text_color) = layout.text_color {
        sheet.typography.body_color = text_color;
        sheet.heading.color = text_color;
    }

    sheet.typography.body_size_pt *= scale;
    for level in 1..=6u8 {
        let resolved_size_pt = sheet.heading.resolve(level).size_pt * scale;
        sheet.heading.levels.entry(level.to_string()).or_default().size_pt = Some(resolved_size_pt);
    }
    sheet.table.text_size_pt *= scale;
    sheet.code_block.default.font_size_pt *= scale;
    for lang_style in sheet.code_block.languages.values_mut() {
        if let Some(size) = lang_style.font_size_pt {
            lang_style.font_size_pt = Some(size * scale);
        }
    }

    sheet
}
