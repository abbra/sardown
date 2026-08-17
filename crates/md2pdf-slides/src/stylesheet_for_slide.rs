use md2pdf_style::{SlideLayoutStyle, Stylesheet};

/// Builds a per-slide `Stylesheet` clone in two ordered steps: first `layout`'s own overrides
/// establish this slide's *unscaled* (scale `1.0`) starting values, then every font-size-bearing
/// field is multiplied by `scale` on top of that. `[header]`/`[footer]` font sizes are never
/// touched by either step -- they're per-document chrome, not slide content, and shrinking a
/// slide's body text must not shrink the page-number footer too.
///
/// This only covers the fields `md2pdf_layout::layout_impl` actually re-reads at render time:
/// `typography.alignment` (paragraph alignment), `typography.body_size_pt`/`.body_color` (list
/// bullet/number markers, built fresh via `marker_inline_node`), `table.text_size_pt` (row-height
/// and padding math around table cells), and `code_block`'s font sizes (resolved fresh per code
/// block via `CodeBlockStyle::resolve`). It deliberately does *not* touch `heading.levels.*` --
/// heading text size/color come from each `BlockNode::Heading`'s own already-parsed
/// `InlineNode.style` fields, baked in once during the original parse and never re-read from this
/// stylesheet during layout, so a heading-scaling override here would silently do nothing. The
/// actual body/heading/table-cell *text* is rescaled directly on the slide's own cloned AST by
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
