mod concat;
mod postprocess;
mod rescale;
mod resolve;
mod shrink;
mod split;
mod stylesheet_for_slide;

pub use concat::concat_slide_layouts;
pub use postprocess::{center_vertically, draw_background_diagram, draw_background_image, fill_background};
use md2pdf_ast::{BlockNode, ImageSource};
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
    let ast = md2pdf_enrich::Highlighter::with_style(stylesheet).highlight(ast);
    let diagrams = md2pdf_enrich::compile_diagrams(&ast);
    // group_columns runs per-slide, not once on the whole deck before splitting: doing it before
    // splitting would let a deck author's forgotten `::end` silently swallow every remaining
    // block in the *entire rest of the deck*, including slides after the next `---`, instead of
    // just the rest of the one slide it appears on.
    let slides: Vec<Slide> =
        split_into_slides(ast).into_iter().map(|slide| Slide { layout_name: slide.layout_name, blocks: md2pdf_ast::group_columns(slide.blocks) }).collect();

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
        // Drawn *before* fill_background: see draw_background_image's own doc comment for why
        // that insertion order produces the correct final paint order (fill, then images, then
        // the slide's own content). Each entry is looked up in both the raster and the SVG
        // table -- whichever one actually decoded the path determines which element kind gets
        // drawn, the same dual-table check `render_block`'s own `BlockNode::Image` arm uses.
        for image in &layout.background_images {
            let synthetic_ast = [BlockNode::Image { alt: String::new(), title: None, source: ImageSource::Embedded(image.path.clone()) }];
            let key = image.path.to_string_lossy().to_string();
            let decoded_table = md2pdf_layout::decode_images(&synthetic_ast, base_dir);
            let diagram_table = md2pdf_layout::collect_svg_diagrams(&synthetic_ast, base_dir);
            let (page_width_pt, page_height_pt) = (output.page_width_pt, output.page_height_pt);
            if let Some(decoded) = decoded_table.get(&key) {
                let height_pt = image.width_pt * (decoded.height as f32 / decoded.width as f32);
                for page in &mut output.pages {
                    draw_background_image(page, &key, image.corner, image.width_pt, height_pt, image.margin_pt, page_width_pt, page_height_pt);
                }
            } else if let Some(diagram) = diagram_table.get(&key) {
                let height_pt = image.width_pt * (diagram.height / diagram.width);
                for page in &mut output.pages {
                    draw_background_diagram(page, &key, image.corner, image.width_pt, height_pt, image.margin_pt, page_width_pt, page_height_pt);
                }
            }
            // If decoding failed, decode_images/collect_svg_diagrams already printed a warning --
            // matches this project's "skip the one broken piece, don't fail the whole render"
            // convention.
            output.images.extend(decoded_table);
            output.diagrams.extend(diagram_table);
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
