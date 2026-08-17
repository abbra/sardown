mod concat;
mod postprocess;
mod rescale;
mod resolve;
mod shrink;
mod split;
mod stylesheet_for_slide;

pub use concat::concat_slide_layouts;
pub use postprocess::{center_vertically, fill_background};
pub use rescale::rescale_slide_content;
pub use resolve::resolve_layout;
pub use shrink::layout_slide_with_shrink;
pub use split::{split_into_slides, Slide};
pub use stylesheet_for_slide::build_slide_stylesheet;

/// Renders a whole slide deck (a Markdown document split into slides on `---`) into one
/// deck-wide `LayoutOutput`, ready for `md2pdf_pdf::render_pdf`. See the design spec
/// (`docs/superpowers/specs/2026-08-17-native-slides-mode-design.md`) for the full pipeline this
/// implements: parse once, split into slides, resolve + auto-shrink + post-process each slide
/// independently, then concatenate and run the existing header/footer and margin passes.
pub fn render_slide_deck(
    markdown: &str,
    base_dir: &std::path::Path,
    font_system: &mut cosmic_text::FontSystem,
    stylesheet: &md2pdf_style::Stylesheet,
) -> anyhow::Result<md2pdf_layout::LayoutOutput> {
    let mut slugs = md2pdf_ast::SlugGenerator::new();
    let mut next_diagram_id = 0usize;
    let ast = md2pdf_ast::parse_with_style(markdown, &mut slugs, &mut next_diagram_id, stylesheet);
    let diagrams = md2pdf_enrich::compile_diagrams(&ast);
    let slides = split_into_slides(ast);

    let (width_mm, height_mm) = stylesheet.page.dimensions_mm();
    let geometry = md2pdf_layout::PageGeometry {
        page_width_mm: width_mm,
        page_height_mm: height_mm,
        margin_mm: stylesheet.page.margin_mm,
        inner_margin_mm: stylesheet.page.inner_margin_mm,
        outer_margin_mm: stylesheet.page.outer_margin_mm,
    };

    let mut slide_outputs = Vec::with_capacity(slides.len());
    for (i, slide) in slides.iter().enumerate() {
        let layout = resolve_layout(slide.layout_name.as_deref(), &stylesheet.slides)?;
        let mut output = layout_slide_with_shrink(
            &slide.blocks,
            &geometry,
            font_system,
            base_dir,
            &diagrams,
            stylesheet,
            &layout,
            stylesheet.slides.min_scale,
            i + 1,
        );

        if layout.vertical_align == md2pdf_style::VerticalAlign::Center {
            let page_height_pt = output.page_height_pt;
            for page in &mut output.pages {
                center_vertically(page, page_height_pt);
            }
        }
        if let Some(color) = layout.background_color {
            let (page_width_pt, page_height_pt) = (output.page_width_pt, output.page_height_pt);
            for page in &mut output.pages {
                fill_background(page, color, page_width_pt, page_height_pt);
            }
        }
        for ctx in &mut output.page_contexts {
            // `layout_impl`'s "chapter opener" heuristic (an H1 heading as the very first thing
            // on a page) is a book-specific concept -- most slides open with an H1 title, which
            // would otherwise suppress the header/footer on nearly every slide by default
            // (`suppress_on_chapter_start` defaults to `true`), independent of and unrelated to
            // this layout's own `suppress_header`/`suppress_footer`. Slides mode has no chapters,
            // so this heuristic is always disabled here -- the resolved layout is the only
            // suppression signal a slide deck gets.
            ctx.is_chapter_opener = false;
            ctx.suppress_header = layout.suppress_header;
            ctx.suppress_footer = layout.suppress_footer;
        }

        slide_outputs.push(output);
    }

    let mut combined = concat_slide_layouts(slide_outputs);
    md2pdf_layout::render_headers_footers(&mut combined.pages, &combined.page_contexts, &combined.anchors, stylesheet, &geometry, font_system);
    md2pdf_layout::apply_asymmetric_margins(&mut combined.pages, &geometry);
    Ok(combined)
}
