use sardown_style::{SlideLayoutStyle, Stylesheet};

/// Builds a per-slide `Stylesheet` clone in two ordered steps: first `layout`'s own overrides
/// establish this slide's *unscaled* (scale `1.0`) starting values, then every font-size-bearing
/// field is multiplied by `scale` on top of that. `[header]`/`[footer]` font sizes are never
/// touched by either step -- they're per-document chrome, not slide content, and shrinking a
/// slide's body text must not shrink the page-number footer too.
///
/// This only covers the fields `sardown_layout::layout_impl` actually re-reads at render time:
/// `typography.alignment` (paragraph alignment), `typography.body_size_pt`/`.body_color` (list
/// bullet/number markers, built fresh via `marker_inline_node`), `table.text_size_pt` (row-height
/// and padding math around table cells), `code_block`'s font sizes (resolved fresh per code block
/// via `CodeBlockStyle::resolve`), and `heading.levels.*.underline_color` (resolved fresh per
/// heading via `HeadingStyle::resolve`). It deliberately does *not* touch heading text size/color
/// -- those come from each `BlockNode::Heading`'s own already-parsed `InlineNode.style` fields,
/// baked in once during the original parse and never re-read from this stylesheet during layout,
/// so a heading-scaling override here would silently do nothing. The actual body/heading/
/// table-cell *text* is rescaled directly on the slide's own cloned AST by
/// `rescale_slide_content`, which this function's sibling module provides -- see that module's
/// doc comment for the full explanation of why both mechanisms are needed together.
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
        // Overwritten per-level (not just the document-wide `heading.underline_color`) because
        // `HeadingStyle::resolve` prefers a level's own override when one is set in `base` --
        // leaving those alone would keep the base document's underline color on any level that
        // configures one, even though this layout wants its own accent color throughout.
        for level in 1..=6u8 {
            sheet.heading.levels.entry(level.to_string()).or_default().underline_color = Some(text_color);
        }
    }

    sheet.typography.body_size_pt *= scale;
    sheet.table.text_size_pt *= scale;
    sheet.code_block.default.font_size_pt *= scale;
    for lang_style in sheet.code_block.languages.values_mut() {
        if let Some(size) = lang_style.font_size_pt {
            lang_style.font_size_pt = Some(size * scale);
        }
    }

    sheet
}

/// The scaling step of [`build_slide_stylesheet`] on its own, applied to an already-overridden
/// sheet in place. Each size-bearing field is set from its own source value (the layout's
/// override where one exists, the base otherwise) *times* `scale` -- never multiplied off
/// whatever is currently stored -- so calling it repeatedly with descending scales across a
/// shrink loop's retries cannot compound. Alignment/color/underline overrides are untouched:
/// they belong to the override step, which is scale-independent and runs once per slide.
///
/// `build_slide_stylesheet(base, layout, scale)` is exactly
/// `let mut s = <overrides applied to base>; apply_slide_scale(&mut s, base, layout, scale)`;
/// this split exists so `layout_slide_with_shrink` can pay for the full stylesheet clone once
/// per slide instead of once per shrink iteration.
pub fn apply_slide_scale(sheet: &mut Stylesheet, base: &Stylesheet, layout: &SlideLayoutStyle, scale: f32) {
    sheet.typography.body_size_pt = layout.body_size_pt.unwrap_or(base.typography.body_size_pt) * scale;
    sheet.table.text_size_pt = base.table.text_size_pt * scale;
    sheet.code_block.default.font_size_pt = base.code_block.default.font_size_pt * scale;
    for (lang, lang_style) in sheet.code_block.languages.iter_mut() {
        if let Some(size) = base.code_block.languages.get(lang).and_then(|s| s.font_size_pt) {
            lang_style.font_size_pt = Some(size * scale);
        }
    }
}
