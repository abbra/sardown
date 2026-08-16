use crate::{PageContext, PageGeometry, PositionedElement, PositionedPage};
use cosmic_text::FontSystem;
use md2pdf_ast::{InlineNode, TextStyle};
use md2pdf_style::{HeaderFooterMode, HeaderFooterStyle, HeaderZones, NumberingFormat, Stylesheet};

// Mirrors paginate.rs's own PT_PER_MM: a fixed physical constant, not business logic, so a small
// local duplicate is lower-risk than threading a cross-module import for it.
const PT_PER_MM: f32 = 2.834645669;

const ROMAN_NUMERALS: [(u32, &str); 13] = [
    (1000, "M"),
    (900, "CM"),
    (500, "D"),
    (400, "CD"),
    (100, "C"),
    (90, "XC"),
    (50, "L"),
    (40, "XL"),
    (10, "X"),
    (9, "IX"),
    (5, "V"),
    (4, "IV"),
    (1, "I"),
];

fn to_roman(mut n: u32) -> String {
    let mut result = String::new();
    for &(value, numeral) in &ROMAN_NUMERALS {
        while n >= value {
            result.push_str(numeral);
            n -= value;
        }
    }
    result
}

/// Formats `n` per `format`. Roman numerals have no representation for 0 and become unwieldy
/// past the conventional range of 1-3999, so both fall back to the plain arabic form -- silently
/// for 0 (an unusual but harmless `start_at = 0` choice), with a warning above 3999.
pub fn format_page_number(n: u32, format: NumberingFormat) -> String {
    match format {
        NumberingFormat::Arabic => n.to_string(),
        NumberingFormat::RomanLower | NumberingFormat::RomanUpper => {
            if n == 0 {
                return n.to_string();
            }
            if n > 3999 {
                eprintln!(
                    "warning: page number {n} exceeds the conventional roman numeral range (1-3999); \
                     falling back to arabic for this value"
                );
                return n.to_string();
            }
            let roman = to_roman(n);
            match format {
                NumberingFormat::RomanLower => roman.to_lowercase(),
                _ => roman,
            }
        }
    }
}

/// Substitutes `{h1}`, `{h2}`, `{page}`, and `{total_pages}` in `template`. Assumes `template`
/// was already validated by `md2pdf_style::Stylesheet::validate` (built in this feature's Phase
/// 1) -- an unknown placeholder or unterminated `{` here indicates a caller bypassed that
/// validation, so this panics rather than silently producing wrong output or duplicating
/// validation logic that already lives in `md2pdf-style`.
pub fn resolve_template(template: &str, ctx: &PageContext, page_display: &str, total_pages_display: &str) -> String {
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
    margin_pt: f32,
    content_width_pt: f32,
    baseline_y: f32,
    is_odd_physical_page: bool,
    font_system: &mut FontSystem,
) {
    let zones = zones_for(style, is_odd_physical_page);
    for (template, align) in [(&zones.left, Align::Left), (&zones.center, Align::Center), (&zones.right, Align::Right)] {
        let resolved = resolve_template(template, ctx, page_display, total_pages_display);
        if resolved.is_empty() {
            continue;
        }
        let node = InlineNode {
            text: resolved,
            style: TextStyle { bold: false, italic: false, size: style.font_size_pt, color: style.color.0 },
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
    stylesheet: &Stylesheet,
    geometry: &PageGeometry,
    font_system: &mut FontSystem,
) {
    if !stylesheet.header.enabled && !stylesheet.footer.enabled {
        return;
    }
    let margin_pt = geometry.margin_mm * PT_PER_MM;
    let content_width_pt = geometry.page_width_mm * PT_PER_MM - 2.0 * margin_pt;
    let full_page_height_pt = geometry.page_height_mm * PT_PER_MM;
    let numbering = &stylesheet.page.numbering;
    let total_pages_display = format_page_number(numbering.start_at + pages.len() as u32 - 1, numbering.format);

    for (i, (page, ctx)) in pages.iter_mut().zip(contexts.iter()).enumerate() {
        let page_display = format_page_number(numbering.start_at + i as u32, numbering.format);
        let is_odd_physical_page = i % 2 == 0;

        if stylesheet.header.enabled && !(stylesheet.header.suppress_on_chapter_start && ctx.is_chapter_opener) {
            render_band(
                page,
                &stylesheet.header,
                ctx,
                &page_display,
                &total_pages_display,
                margin_pt,
                content_width_pt,
                margin_pt * 0.6,
                is_odd_physical_page,
                font_system,
            );
        }
        if stylesheet.footer.enabled && !(stylesheet.footer.suppress_on_chapter_start && ctx.is_chapter_opener) {
            render_band(
                page,
                &stylesheet.footer,
                ctx,
                &page_display,
                &total_pages_display,
                margin_pt,
                content_width_pt,
                full_page_height_pt - margin_pt * 0.6,
                is_odd_physical_page,
                font_system,
            );
        }
    }
}

pub fn layout_with_header_footer(
    ast: &[md2pdf_ast::BlockNode],
    geometry: &PageGeometry,
    font_system: &mut FontSystem,
    base_dir: &std::path::Path,
    diagrams: &md2pdf_enrich::DiagramTable,
    stylesheet: &Stylesheet,
) -> crate::LayoutOutput {
    let mut output = crate::layout(ast, geometry, font_system, base_dir, diagrams);
    render_headers_footers(&mut output.pages, &output.page_contexts, stylesheet, geometry, font_system);
    output
}
