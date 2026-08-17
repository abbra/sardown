use crate::rescale::rescale_slide_content;
use crate::stylesheet_for_slide::build_slide_stylesheet;
use cosmic_text::FontSystem;
use md2pdf_ast::BlockNode;
use md2pdf_enrich::DiagramTable;
use md2pdf_layout::{layout_impl, LayoutOutput, PageGeometry};
use md2pdf_style::{SlideLayoutStyle, Stylesheet};

const SCALE_STEP: f32 = 0.05;

/// Lays out one slide's blocks, retrying at successively smaller font-size scales (starting at
/// `1.0`, stepping down by `SCALE_STEP`) until the result fits on one page or `min_scale` is
/// reached -- the floor scale is always actually tried, never skipped. If even the floor scale
/// still overflows, returns whatever `layout_impl` produced there in full: overflow content is
/// never dropped, even though it means this one slide breaks the "one slide, one page" invariant
/// every other slide holds.
///
/// Each retry works against a *fresh* clone of `blocks`, rescaled via `rescale_slide_content` --
/// `layout_impl` alone doesn't shrink already-parsed body/heading/table-cell text no matter what
/// `Stylesheet` it's given (see `rescale_slide_content`'s doc comment), so this is the mechanism
/// that actually makes each retry's smaller scale visible in the rendered output.
#[allow(clippy::too_many_arguments)]
pub fn layout_slide_with_shrink(
    blocks: &[BlockNode],
    geometry: &PageGeometry,
    font_system: &mut FontSystem,
    base_dir: &std::path::Path,
    diagrams: &DiagramTable,
    base_stylesheet: &Stylesheet,
    layout: &SlideLayoutStyle,
    min_scale: f32,
    slide_number: usize,
) -> LayoutOutput {
    let mut scale = 1.0f32;
    loop {
        let mut attempt_blocks = blocks.to_vec();
        rescale_slide_content(&mut attempt_blocks, base_stylesheet, layout, scale);
        let slide_stylesheet = build_slide_stylesheet(base_stylesheet, layout, scale);
        let output = layout_impl(&attempt_blocks, geometry, font_system, base_dir, diagrams, &slide_stylesheet);
        let fits = output.pages.len() <= 1;
        let at_floor = scale <= min_scale;
        if fits || at_floor {
            if !fits {
                let heading = first_heading_text(blocks).map(|h| format!(" ({h:?})")).unwrap_or_default();
                eprintln!(
                    "warning: slide {slide_number}{heading} still does not fit on one page at the minimum \
                     scale ({min_scale}); rendering all {} pages of its content instead of dropping any",
                    output.pages.len()
                );
            }
            return output;
        }
        scale = (scale - SCALE_STEP).max(min_scale);
    }
}

fn first_heading_text(blocks: &[BlockNode]) -> Option<String> {
    blocks.iter().find_map(|b| match b {
        BlockNode::Heading { content, .. } => Some(content.iter().map(|n| n.text.as_str()).collect()),
        _ => None,
    })
}
