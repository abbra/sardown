use crate::numbering::{display_number_for_page, format_page_number, resolve_numbering_segments};
use crate::{AnchorTable, PageContext, PageGeometry, PositionedElement, PositionedPage};
use cosmic_text::FontSystem;
use md2pdf_ast::{InlineNode, TextStyle};
use md2pdf_style::{DocumentStyle, HeaderFooterMode, HeaderFooterStyle, HeaderZones, Stylesheet};

// Mirrors paginate.rs's own PT_PER_MM: a fixed physical constant, not business logic, so a small
// local duplicate is lower-risk than threading a cross-module import for it.
const PT_PER_MM: f32 = 2.834645669;

/// Substitutes `{h1}`, `{h2}`, `{page}`, `{total_pages}`, `{title}`, `{author}`, and `{date}` in
/// `template`.
/// Assumes `template` was already validated by `md2pdf_style::Stylesheet::validate` (built in
/// this feature's Phase 1) -- an unknown placeholder or unterminated `{` here indicates a caller
/// bypassed that validation, so this panics rather than silently producing wrong output or
/// duplicating validation logic that already lives in `md2pdf-style`.
pub fn resolve_template(template: &str, ctx: &PageContext, page_display: &str, total_pages_display: &str, document: &DocumentStyle) -> String {
    let mut result = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 1..];
        let end = after_open.find('}').expect("template placeholders are validated before reaching resolve_template");
        let name = &after_open[..end];
        let value = match name {
            "h1" => ctx.current_h1.as_deref().unwrap_or(""),
            "h2" => ctx.current_h2.as_deref().unwrap_or(""),
            "page" => page_display,
            "total_pages" => total_pages_display,
            "title" => document.title.as_str(),
            "author" => document.author.as_str(),
            "date" => document.date.as_str(),
            other => panic!("unknown placeholder {{{other}}} should have been rejected by Stylesheet::validate"),
        };
        result.push_str(value);
        rest = &after_open[end + 1..];
    }
    result.push_str(rest);
    result
}

enum Align {
    Left,
    Center,
    Right,
}

fn zones_for(style: &HeaderFooterStyle, is_odd_physical_page: bool) -> &HeaderZones {
    match style.mode {
        HeaderFooterMode::Uniform => &style.uniform,
        HeaderFooterMode::TwoSided => {
            if is_odd_physical_page {
                &style.odd
            } else {
                &style.even
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_band(
    page: &mut PositionedPage,
    style: &HeaderFooterStyle,
    ctx: &PageContext,
    page_display: &str,
    total_pages_display: &str,
    document: &DocumentStyle,
    margin_pt: f32,
    content_width_pt: f32,
    baseline_y: f32,
    is_odd_physical_page: bool,
    font_system: &mut FontSystem,
) {
    let zones = zones_for(style, is_odd_physical_page);
    for (template, align) in [(&zones.left, Align::Left), (&zones.center, Align::Center), (&zones.right, Align::Right)] {
        let resolved = resolve_template(template, ctx, page_display, total_pages_display, document);
        if resolved.is_empty() {
            continue;
        }
        let node = InlineNode {
            text: resolved,
            style: TextStyle {
                bold: false,
                italic: false,
                strikethrough: false,
                size: style.font_size_pt,
                color: style.color.0,
                font_family: style.font_family.clone(),
            },
            link_target: None,
        };
        // Only the first shaped line is ever used: header/footer zones are expected to be short
        // enough not to wrap within the full content width, and the alignment math below only
        // measures that first line anyway. If a template resolves to something long enough to
        // wrap, taking every wrapped line would place them all at the same baseline_y and overlap
        // -- keeping only the first line is a deliberate, graceful truncation instead.
        let elements = crate::shape_paragraph(font_system, &[node], content_width_pt);
        let Some(mut first) = elements.into_iter().next() else { continue };
        let PositionedElement::TextRun { x, y, glyphs, .. } = &mut first else { continue };
        let text_width: f32 = glyphs.iter().map(|g| g.x_advance).sum();
        *x = match align {
            Align::Left => margin_pt,
            Align::Center => margin_pt + (content_width_pt - text_width) / 2.0,
            Align::Right => margin_pt + content_width_pt - text_width,
        };
        *y = baseline_y;
        page.elements.push(first);
    }
}

pub fn render_headers_footers(
    pages: &mut [PositionedPage],
    contexts: &[PageContext],
    anchors: &AnchorTable,
    stylesheet: &Stylesheet,
    geometry: &PageGeometry,
    font_system: &mut FontSystem,
) {
    if !stylesheet.header.enabled && !stylesheet.footer.enabled {
        return;
    }
    let margin_pt = geometry.margin_mm * PT_PER_MM;
    let content_width_pt = geometry.page_width_mm * PT_PER_MM - geometry.horizontal_margin_budget_mm() * PT_PER_MM;
    let full_page_height_pt = geometry.page_height_mm * PT_PER_MM;
    let numbering = &stylesheet.page.numbering;
    // "Total pages" is always the document's literal physical length in the base numbering
    // format, regardless of any resets -- resets restart the *displayed* count partway through,
    // but the document doesn't have multiple "totals".
    let total_pages_display = format_page_number(numbering.start_at + pages.len() as u32 - 1, numbering.format);
    let segments = resolve_numbering_segments(numbering, anchors);

    for (i, (page, ctx)) in pages.iter_mut().zip(contexts.iter()).enumerate() {
        let page_display = display_number_for_page(i, &segments);
        let is_odd_physical_page = i % 2 == 0;

        if stylesheet.header.enabled && !(stylesheet.header.suppress_on_chapter_start && ctx.is_chapter_opener) && !ctx.suppress_header {
            render_band(
                page,
                &stylesheet.header,
                ctx,
                &page_display,
                &total_pages_display,
                &stylesheet.document,
                margin_pt,
                content_width_pt,
                margin_pt * 0.6,
                is_odd_physical_page,
                font_system,
            );
        }
        if stylesheet.footer.enabled && !(stylesheet.footer.suppress_on_chapter_start && ctx.is_chapter_opener) && !ctx.suppress_footer {
            render_band(
                page,
                &stylesheet.footer,
                ctx,
                &page_display,
                &total_pages_display,
                &stylesheet.document,
                margin_pt,
                content_width_pt,
                full_page_height_pt - margin_pt * 0.6,
                is_odd_physical_page,
                font_system,
            );
        }
    }
}

/// Shifts every element on each page to its real final left edge once `geometry.inner_margin_mm`
/// / `outer_margin_mm` are both set -- a no-op otherwise. Layout itself always places content
/// using plain `margin_mm` as an arbitrary x-origin baseline (see `PageGeometry`'s doc comment for
/// why only `content_width_pt` needs to change during layout); this is the step that moves each
/// page's content from that baseline to `inner_margin_mm` (recto, odd physical pages) or
/// `outer_margin_mm` (verso, even physical pages), matching `render_headers_footers`'s own
/// odd/even physical-page convention.
pub fn apply_asymmetric_margins(pages: &mut [PositionedPage], geometry: &PageGeometry) {
    let (Some(inner_mm), Some(outer_mm)) = (geometry.inner_margin_mm, geometry.outer_margin_mm) else {
        return;
    };
    let baseline_pt = geometry.margin_mm * PT_PER_MM;
    let inner_pt = inner_mm * PT_PER_MM;
    let outer_pt = outer_mm * PT_PER_MM;

    for (i, page) in pages.iter_mut().enumerate() {
        let is_recto = i % 2 == 0;
        let target_left_pt = if is_recto { inner_pt } else { outer_pt };
        let shift_pt = target_left_pt - baseline_pt;
        if shift_pt != 0.0 {
            for element in &mut page.elements {
                crate::shift_element(element, shift_pt, 0.0);
            }
        }
    }
}

pub fn layout_with_header_footer(
    ast: &[md2pdf_ast::BlockNode],
    font_system: &mut FontSystem,
    base_dir: &std::path::Path,
    diagrams: &md2pdf_enrich::DiagramTable,
    stylesheet: &Stylesheet,
) -> crate::LayoutOutput {
    let (width_mm, height_mm) = stylesheet.page.dimensions_mm();
    let geometry = PageGeometry {
        page_width_mm: width_mm,
        page_height_mm: height_mm,
        margin_mm: stylesheet.page.margin_mm,
        inner_margin_mm: stylesheet.page.inner_margin_mm,
        outer_margin_mm: stylesheet.page.outer_margin_mm,
    };
    let mut output = crate::layout_impl(ast, &geometry, font_system, base_dir, diagrams, stylesheet);
    crate::toc::insert_table_of_contents(&mut output, ast, stylesheet, &geometry, font_system);
    render_headers_footers(&mut output.pages, &output.page_contexts, &output.anchors, stylesheet, &geometry, font_system);
    apply_asymmetric_margins(&mut output.pages, &geometry);
    output
}
