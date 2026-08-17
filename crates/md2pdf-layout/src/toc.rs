use crate::{shape_paragraph, LayoutOutput, PageContext, PageGeometry, PositionedElement, PositionedPage, Rect};
use cosmic_text::FontSystem;
use md2pdf_ast::{BlockNode, InlineNode, LinkTarget, TextStyle};
use md2pdf_style::Stylesheet;

const PT_PER_MM: f32 = 2.834645669;
const ENTRY_INDENT_PT: f32 = 18.0;
const DOT_GAP_PADDING_PT: f32 = 4.0;

#[derive(Debug, Clone)]
pub struct TocEntry {
    pub level: u8,
    pub id: String,
    pub text: String,
}

fn collect_entries(ast: &[BlockNode], depth: u8) -> Vec<TocEntry> {
    ast.iter()
        .filter_map(|block| match block {
            BlockNode::Heading { level, id, content } if *level <= depth => Some(TocEntry {
                level: *level,
                id: id.clone(),
                text: content.iter().map(|n| n.text.as_str()).collect(),
            }),
            _ => None,
        })
        .collect()
}

/// Shapes `text` as a single line and returns both the positioned element (with `x`/`y` set to
/// 0.0 -- callers reposition before pushing) and its total glyph-advance width, so the same
/// shape call serves both measurement and placement instead of shaping twice.
fn shape_line(font_system: &mut FontSystem, text: &str, size: f32, color: [u8; 3], font_family: &str) -> (PositionedElement, f32) {
    let node = InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size, color, font_family: font_family.to_string() },
        link_target: None,
    };
    let elements = shape_paragraph(font_system, std::slice::from_ref(&node), f32::MAX);
    let element = elements.into_iter().next().unwrap_or(PositionedElement::TextRun {
        x: 0.0,
        y: 0.0,
        glyphs: Vec::new(),
        text: String::new(),
        font_id: font_system.db().faces().next().expect("no fonts loaded in font_system").id,
        size,
        color,
    });
    let width = match &element {
        PositionedElement::TextRun { glyphs, .. } => glyphs.iter().map(|g| g.x_advance).sum(),
        _ => 0.0,
    };
    (element, width)
}

fn reposition(mut element: PositionedElement, x: f32, y: f32) -> PositionedElement {
    if let PositionedElement::TextRun { x: ex, y: ey, .. } = &mut element {
        *ex = x;
        *ey = y;
    }
    element
}

pub fn insert_table_of_contents(output: &mut LayoutOutput, ast: &[BlockNode], stylesheet: &Stylesheet, geometry: &PageGeometry, font_system: &mut FontSystem) {
    if !stylesheet.toc.enabled {
        return;
    }
    let entries = collect_entries(ast, stylesheet.toc.depth);
    if entries.is_empty() {
        return;
    }

    let margin_pt = geometry.margin_mm * PT_PER_MM;
    let content_width_pt = output.page_width_pt - 2.0 * margin_pt;
    let usable_height_pt = (output.page_height_pt - 2.0 * margin_pt).max(0.0);
    let body = &stylesheet.typography;
    let line_height_pt = crate::paginate::estimate_line_height(body.body_size_pt);
    let lines_per_toc_page = ((usable_height_pt / line_height_pt).floor() as usize).max(1);
    let total_lines = 1 + entries.len();
    let toc_page_count = (total_lines + lines_per_toc_page - 1) / lines_per_toc_page;

    // Shift every existing page and anchor *before* laying out the TOC's own content, so each
    // entry can look up its target heading's already-correct, final page number directly -- this
    // avoids a reflow fixpoint (the TOC's own page count depends only on line count, never on the
    // width of the page-number text next to it, so it can be computed once, up front).
    for anchor in output.anchors.values_mut() {
        anchor.page += toc_page_count;
    }
    for page in &mut output.pages {
        page.page_number += toc_page_count;
    }

    let title_style = stylesheet.heading.resolve(1);
    let (_, dot_width) = shape_line(font_system, ".", body.body_size_pt, body.body_color.0, &body.font_family);
    let dot_width = dot_width.max(0.1);

    let mut toc_pages: Vec<PositionedPage> = Vec::with_capacity(toc_page_count);
    let mut current: Vec<PositionedElement> = Vec::new();
    let mut line_in_page = 0usize;
    let mut y = margin_pt;

    let (title_element, _) = shape_line(font_system, &stylesheet.toc.title, title_style.size_pt, title_style.color.0, &title_style.font_family);
    current.push(reposition(title_element, margin_pt, y));
    line_in_page += 1;

    for entry in &entries {
        if line_in_page >= lines_per_toc_page {
            toc_pages.push(PositionedPage { page_number: toc_pages.len(), elements: std::mem::take(&mut current) });
            y = margin_pt;
            line_in_page = 0;
        } else {
            y += line_height_pt;
        }

        let indent_pt = entry.level.saturating_sub(1) as f32 * ENTRY_INDENT_PT;
        let final_page = output.anchors.get(&entry.id).map(|a| a.page).unwrap_or(0);
        let page_number_text = crate::format_page_number(stylesheet.page.numbering.start_at + final_page as u32, stylesheet.page.numbering.format);

        let (heading_element, heading_width) = shape_line(font_system, &entry.text, body.body_size_pt, body.body_color.0, &body.font_family);
        let (page_number_element, page_number_width) = shape_line(font_system, &page_number_text, body.body_size_pt, body.body_color.0, &body.font_family);

        let heading_x = margin_pt + indent_pt;
        let page_number_x = margin_pt + content_width_pt - page_number_width;
        let dots_start_x = heading_x + heading_width + DOT_GAP_PADDING_PT;
        let dots_end_x = (page_number_x - DOT_GAP_PADDING_PT).max(dots_start_x);
        let dot_count = ((dots_end_x - dots_start_x) / dot_width).floor().max(0.0) as usize;

        current.push(reposition(heading_element, heading_x, y));
        if dot_count > 0 {
            let (dots_element, _) = shape_line(font_system, &".".repeat(dot_count), body.body_size_pt, body.body_color.0, &body.font_family);
            current.push(reposition(dots_element, dots_start_x, y));
        }
        current.push(reposition(page_number_element, page_number_x, y));
        current.push(PositionedElement::LinkAnnotation {
            rect: Rect { x: heading_x, y: y - body.body_size_pt, width: content_width_pt - indent_pt, height: body.body_size_pt * 1.2 },
            destination: LinkTarget::InternalAnchor(entry.id.clone()),
        });

        line_in_page += 1;
    }
    toc_pages.push(PositionedPage { page_number: toc_pages.len(), elements: current });

    debug_assert_eq!(toc_pages.len(), toc_page_count, "TOC page count arithmetic and actual pagination must agree");

    let default_context = || PageContext { current_h1: None, current_h2: None, is_chapter_opener: false };
    output.page_contexts.splice(0..0, std::iter::repeat_with(default_context).take(toc_pages.len()));
    output.pages.splice(0..0, toc_pages);
    output.toc_entries = entries;
}
