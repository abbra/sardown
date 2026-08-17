use cosmic_text::FontSystem;
use md2pdf_ast::{BlockNode, InlineNode, TextStyle};
use md2pdf_enrich::DiagramTable;
use md2pdf_layout::{
    layout_with_header_footer, render_headers_footers, AnchorPosition, AnchorTable, PageContext, PageGeometry, PositionedElement, PositionedPage,
};
use md2pdf_style::{HeaderFooterMode, Stylesheet};

fn test_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).expect("failed to load test font");
    db.set_sans_serif_family("Droid Sans");
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

fn geometry() -> PageGeometry {
    PageGeometry { page_width_mm: 215.9, page_height_mm: 279.4, margin_mm: 25.4, ..Default::default() }
}

fn empty_page(n: usize) -> PositionedPage {
    PositionedPage { page_number: n, elements: Vec::new() }
}

fn ctx(h1: Option<&str>, is_chapter_opener: bool) -> PageContext {
    PageContext { current_h1: h1.map(String::from), current_h2: None, is_chapter_opener }
}

fn text_of(page: &PositionedPage) -> Vec<String> {
    page.elements
        .iter()
        .filter_map(|e| match e {
            PositionedElement::TextRun { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn uniform_header_renders_the_resolved_center_template() {
    let mut sheet = Stylesheet::default();
    sheet.header.enabled = true;
    sheet.header.uniform.center = "{h1}".to_string();
    let mut pages = vec![empty_page(0)];
    let contexts = vec![ctx(Some("Chapter One"), false)];
    let mut fs = test_font_system();
    render_headers_footers(&mut pages, &contexts, &AnchorTable::new(), &sheet, &geometry(), &mut fs);
    assert!(text_of(&pages[0]).contains(&"Chapter One".to_string()));
}

#[test]
fn header_renders_the_document_title_and_author() {
    let mut sheet = Stylesheet::default();
    sheet.document.title = "My Book".to_string();
    sheet.document.author = "Jane Doe".to_string();
    sheet.header.enabled = true;
    sheet.header.uniform.left = "{title}".to_string();
    sheet.header.uniform.right = "{author}".to_string();
    let mut pages = vec![empty_page(0)];
    let contexts = vec![ctx(None, false)];
    let mut fs = test_font_system();
    render_headers_footers(&mut pages, &contexts, &AnchorTable::new(), &sheet, &geometry(), &mut fs);
    let text = text_of(&pages[0]);
    assert!(text.contains(&"My Book".to_string()));
    assert!(text.contains(&"Jane Doe".to_string()));
}

#[test]
fn footer_renders_page_number_and_total() {
    let mut sheet = Stylesheet::default();
    sheet.footer.enabled = true;
    sheet.footer.uniform.center = "Page {page} of {total_pages}".to_string();
    let mut pages = vec![empty_page(0), empty_page(1)];
    let contexts = vec![ctx(None, false), ctx(None, false)];
    let mut fs = test_font_system();
    render_headers_footers(&mut pages, &contexts, &AnchorTable::new(), &sheet, &geometry(), &mut fs);
    assert!(text_of(&pages[0]).contains(&"Page 1 of 2".to_string()));
    assert!(text_of(&pages[1]).contains(&"Page 2 of 2".to_string()));
}

#[test]
fn a_numbering_reset_restarts_the_format_and_count_from_its_heading_onward() {
    use md2pdf_style::NumberingFormat;
    let mut sheet = Stylesheet::default();
    sheet.page.numbering.format = NumberingFormat::RomanLower;
    sheet.page.numbering.resets =
        vec![md2pdf_style::PageNumberingReset { at_heading: "chapter-one".to_string(), format: NumberingFormat::Arabic, start_at: 1 }];
    sheet.footer.enabled = true;
    sheet.footer.uniform.center = "{page}".to_string();

    let mut anchors = AnchorTable::new();
    anchors.insert("chapter-one".to_string(), AnchorPosition { page: 2, x: 0.0, y: 0.0 });

    // Pages 0-1: front matter, roman_lower (i, ii). Page 2 onward: reset to arabic starting at 1.
    let mut pages = vec![empty_page(0), empty_page(1), empty_page(2), empty_page(3)];
    let contexts = vec![ctx(None, false), ctx(None, false), ctx(None, false), ctx(None, false)];
    let mut fs = test_font_system();
    render_headers_footers(&mut pages, &contexts, &anchors, &sheet, &geometry(), &mut fs);

    assert!(text_of(&pages[0]).contains(&"i".to_string()));
    assert!(text_of(&pages[1]).contains(&"ii".to_string()));
    assert!(text_of(&pages[2]).contains(&"1".to_string()));
    assert!(text_of(&pages[3]).contains(&"2".to_string()));
}

#[test]
fn a_reset_naming_an_unknown_heading_is_ignored() {
    use md2pdf_style::NumberingFormat;
    let mut sheet = Stylesheet::default();
    sheet.page.numbering.resets =
        vec![md2pdf_style::PageNumberingReset { at_heading: "no-such-heading".to_string(), format: NumberingFormat::RomanUpper, start_at: 5 }];
    sheet.footer.enabled = true;
    sheet.footer.uniform.center = "{page}".to_string();
    let mut pages = vec![empty_page(0)];
    let contexts = vec![ctx(None, false)];
    let mut fs = test_font_system();
    render_headers_footers(&mut pages, &contexts, &AnchorTable::new(), &sheet, &geometry(), &mut fs);
    assert!(text_of(&pages[0]).contains(&"1".to_string()), "expected the base numbering unaffected by an unresolvable reset");
}

#[test]
fn disabled_header_and_footer_add_no_elements() {
    let sheet = Stylesheet::default(); // enabled = false by default
    let mut pages = vec![empty_page(0)];
    let contexts = vec![ctx(None, false)];
    let mut fs = test_font_system();
    render_headers_footers(&mut pages, &contexts, &AnchorTable::new(), &sheet, &geometry(), &mut fs);
    assert!(pages[0].elements.is_empty());
}

#[test]
fn an_empty_resolved_zone_adds_no_element() {
    let mut sheet = Stylesheet::default();
    sheet.header.enabled = true; // left/center/right all default to ""
    let mut pages = vec![empty_page(0)];
    let contexts = vec![ctx(None, false)];
    let mut fs = test_font_system();
    render_headers_footers(&mut pages, &contexts, &AnchorTable::new(), &sheet, &geometry(), &mut fs);
    assert!(pages[0].elements.is_empty());
}

#[test]
fn left_zone_is_positioned_at_the_margin() {
    let mut sheet = Stylesheet::default();
    sheet.header.enabled = true;
    sheet.header.uniform.left = "L".to_string();
    let mut pages = vec![empty_page(0)];
    let contexts = vec![ctx(None, false)];
    let mut fs = test_font_system();
    render_headers_footers(&mut pages, &contexts, &AnchorTable::new(), &sheet, &geometry(), &mut fs);
    let margin_pt = 25.4 * 2.834645669;
    let x = pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { x, .. } => Some(*x),
            _ => None,
        })
        .expect("expected a text run");
    assert!((x - margin_pt).abs() < 0.5, "expected left zone near margin ({margin_pt}), got {x}");
}

#[test]
fn right_zone_is_right_aligned_to_the_content_edge() {
    let mut sheet = Stylesheet::default();
    sheet.header.enabled = true;
    sheet.header.uniform.right = "R".to_string();
    let mut pages = vec![empty_page(0)];
    let contexts = vec![ctx(None, false)];
    let mut fs = test_font_system();
    render_headers_footers(&mut pages, &contexts, &AnchorTable::new(), &sheet, &geometry(), &mut fs);
    let margin_pt = 25.4 * 2.834645669;
    let content_width_pt = 215.9 * 2.834645669 - 2.0 * margin_pt;
    let right_edge = margin_pt + content_width_pt;
    let (x, glyphs_width) = pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { x, glyphs, .. } => Some((*x, glyphs.iter().map(|g| g.x_advance).sum::<f32>())),
            _ => None,
        })
        .expect("expected a text run");
    assert!((x + glyphs_width - right_edge).abs() < 0.5, "expected right zone's end to align with the content edge ({right_edge}), got {}", x + glyphs_width);
}

#[test]
fn header_is_suppressed_on_a_chapter_opener_page_by_default() {
    let mut sheet = Stylesheet::default();
    sheet.header.enabled = true;
    sheet.header.uniform.center = "{h1}".to_string();
    let mut pages = vec![empty_page(0)];
    let contexts = vec![ctx(Some("Chapter One"), true)];
    let mut fs = test_font_system();
    render_headers_footers(&mut pages, &contexts, &AnchorTable::new(), &sheet, &geometry(), &mut fs);
    assert!(pages[0].elements.is_empty());
}

#[test]
fn header_still_renders_on_a_chapter_opener_when_suppression_is_disabled() {
    let mut sheet = Stylesheet::default();
    sheet.header.enabled = true;
    sheet.header.suppress_on_chapter_start = false;
    sheet.header.uniform.center = "{h1}".to_string();
    let mut pages = vec![empty_page(0)];
    let contexts = vec![ctx(Some("Chapter One"), true)];
    let mut fs = test_font_system();
    render_headers_footers(&mut pages, &contexts, &AnchorTable::new(), &sheet, &geometry(), &mut fs);
    assert!(!pages[0].elements.is_empty());
}

#[test]
fn two_sided_mode_uses_odd_zones_on_the_first_physical_page() {
    let mut sheet = Stylesheet::default();
    sheet.header.enabled = true;
    sheet.header.mode = HeaderFooterMode::TwoSided;
    sheet.header.odd.left = "ODD".to_string();
    sheet.header.even.left = "EVEN".to_string();
    let mut pages = vec![empty_page(0)];
    let contexts = vec![ctx(None, false)];
    let mut fs = test_font_system();
    render_headers_footers(&mut pages, &contexts, &AnchorTable::new(), &sheet, &geometry(), &mut fs);
    assert!(text_of(&pages[0]).contains(&"ODD".to_string()));
}

#[test]
fn two_sided_mode_uses_even_zones_on_the_second_physical_page() {
    let mut sheet = Stylesheet::default();
    sheet.header.enabled = true;
    sheet.header.mode = HeaderFooterMode::TwoSided;
    sheet.header.odd.left = "ODD".to_string();
    sheet.header.even.left = "EVEN".to_string();
    let mut pages = vec![empty_page(0), empty_page(1)];
    let contexts = vec![ctx(None, false), ctx(None, false)];
    let mut fs = test_font_system();
    render_headers_footers(&mut pages, &contexts, &AnchorTable::new(), &sheet, &geometry(), &mut fs);
    assert!(text_of(&pages[1]).contains(&"EVEN".to_string()));
}

fn fixtures_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn plain_inline(text: &str) -> InlineNode {
    InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size: 12.0, color: [0, 0, 0], font_family: "sans-serif".to_string() },
        link_target: None,
    }
}

#[test]
fn layout_with_header_footer_renders_end_to_end_across_a_forced_page_break() {
    let ast = vec![
        BlockNode::Heading { level: 1, id: "ch1".to_string(), content: vec![plain_inline("Chapter One")] },
        BlockNode::Paragraph { content: vec![plain_inline("Body of chapter one.")] },
        BlockNode::PageBreak,
        BlockNode::Heading { level: 1, id: "ch2".to_string(), content: vec![plain_inline("Chapter Two")] },
        BlockNode::Paragraph { content: vec![plain_inline("Body of chapter two.")] },
    ];
    let mut sheet = Stylesheet::default();
    sheet.header.enabled = true;
    sheet.header.uniform.center = "{h1}".to_string();
    sheet.footer.enabled = true;
    sheet.footer.suppress_on_chapter_start = false; // both pages here are chapter openers; keep
                                                    // the footer's own numbering independently
                                                    // observable from the header's suppression
    sheet.footer.uniform.center = "Page {page} of {total_pages}".to_string();
    let mut fs = test_font_system();
    let output = layout_with_header_footer(&ast, &mut fs, &fixtures_dir(), &DiagramTable::new(), &sheet);

    assert_eq!(output.pages.len(), 2);
    // Both pages open with their own chapter's H1, so the header (suppressed on chapter openers
    // by default) is absent on both.
    // "Chapter One"/"Chapter Two" each legitimately appear once already, as the document's own
    // rendered heading text -- a suppressed header must not add a *second* occurrence.
    let count = |texts: &[String], needle: &str| texts.iter().filter(|t| t.as_str() == needle).count();
    assert_eq!(count(&text_of(&output.pages[0]), "Chapter One"), 1, "expected only the body heading, no extra header copy");
    assert_eq!(count(&text_of(&output.pages[1]), "Chapter Two"), 1, "expected only the body heading, no extra header copy");
    assert!(text_of(&output.pages[0]).contains(&"Page 1 of 2".to_string()));
    assert!(text_of(&output.pages[1]).contains(&"Page 2 of 2".to_string()));
}
