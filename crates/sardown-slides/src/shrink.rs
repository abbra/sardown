use crate::rescale::rescale_slide_content;
use crate::stylesheet_for_slide::build_slide_stylesheet;
use cosmic_text::FontSystem;
use sardown_ast::BlockNode;
use sardown_layout::{layout_with_assets, LayoutAssets, LayoutOutput, PageGeometry};
use sardown_style::{SlideLayoutStyle, Stylesheet};

const SCALE_STEP: f32 = 0.05;

/// Context shared identically across every slide in one deck's own `layout_slide_with_shrink`
/// call -- the page geometry, the deck-wide pre-decoded layout assets (images, diagrams,
/// hyphenator), the document-wide base stylesheet, and the auto-shrink floor scale. Grouped into
/// one value so the per-slide function itself only takes the things that actually vary per call.
///
/// `assets` is prepared ONCE per deck (see `sardown_layout::prepare_layout_assets` and
/// `LayoutAssets`): the auto-shrink loop below retries each slide's layout at successively
/// smaller scales, and every retry reuses the same decoded images / parsed SVGs / hyphenation
/// dictionary instead of re-decoding and re-parsing them from scratch on every attempt.
pub struct DeckContext<'a> {
    pub geometry: &'a PageGeometry,
    pub assets: &'a LayoutAssets,
    pub base_stylesheet: &'a Stylesheet,
    pub min_scale: f32,
}

/// Lays out one slide's blocks, retrying at successively smaller font-size scales (starting at
/// `1.0`, stepping down by `SCALE_STEP`) until the result fits on one page or `deck.min_scale` is
/// reached -- the floor scale is always actually tried, never skipped. If even the floor scale
/// still overflows, returns whatever `layout_impl` produced there in full: overflow content is
/// never dropped, even though it means this one slide breaks the "one slide, one page" invariant
/// every other slide holds.
///
/// Each retry works against a *fresh* clone of `blocks`, rescaled via `rescale_slide_content` --
/// `layout_impl` alone doesn't shrink already-parsed body/heading/table-cell text no matter what
/// `Stylesheet` it's given (see `rescale_slide_content`'s doc comment), so this is the mechanism
/// that actually makes each retry's smaller scale visible in the rendered output.
pub fn layout_slide_with_shrink(
    blocks: &[BlockNode],
    font_system: &mut FontSystem,
    deck: &DeckContext,
    layout: &SlideLayoutStyle,
    slide_number: usize,
) -> LayoutOutput {
    let mut scale = 1.0f32;
    loop {
        let mut attempt_blocks = blocks.to_vec();
        rescale_slide_content(&mut attempt_blocks, deck.base_stylesheet, layout, scale);
        let slide_stylesheet = build_slide_stylesheet(deck.base_stylesheet, layout, scale);
        let output = layout_with_assets(&attempt_blocks, deck.geometry, font_system, deck.assets, &slide_stylesheet);
        let fits = output.pages.len() <= 1;
        let at_floor = scale <= deck.min_scale;
        if fits || at_floor {
            if !fits {
                let heading = first_heading_text(blocks).map(|h| format!(" ({h:?})")).unwrap_or_default();
                eprintln!(
                    "warning: slide {slide_number}{heading} still does not fit on one page at the minimum \
                     scale ({}); rendering all {} pages of its content instead of dropping any",
                    deck.min_scale,
                    output.pages.len()
                );
            }
            return output;
        }
        scale = (scale - SCALE_STEP).max(deck.min_scale);
    }
}

fn first_heading_text(blocks: &[BlockNode]) -> Option<String> {
    blocks.iter().find_map(|b| match b {
        BlockNode::Heading { content, .. } => Some(content.iter().map(|n| n.text.as_str()).collect()),
        _ => None,
    })
}
