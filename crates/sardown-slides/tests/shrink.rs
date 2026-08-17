use cosmic_text::FontSystem;
use sardown_ast::{BlockNode, InlineNode, TextStyle};
use sardown_enrich::DiagramTable;
use sardown_layout::PageGeometry;
use sardown_slides::{layout_slide_with_shrink, DeckContext};
use sardown_style::{SlideLayoutStyle, Stylesheet};

fn test_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).unwrap();
    db.set_sans_serif_family("Droid Sans");
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

fn geometry() -> PageGeometry {
    PageGeometry { page_width_mm: 100.0, page_height_mm: 60.0, margin_mm: 5.0, ..Default::default() }
}

fn deck_context<'a>(geometry: &'a PageGeometry, diagrams: &'a DiagramTable, base_stylesheet: &'a Stylesheet) -> DeckContext<'a> {
    DeckContext { geometry, base_dir: std::path::Path::new("."), diagrams, base_stylesheet, min_scale: 0.5 }
}

fn plain(text: &str) -> InlineNode {
    InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size: 12.0, color: [0, 0, 0], font_family: "sans-serif".to_string() },
        link_target: None,
    }
}

fn paragraph(text: &str) -> BlockNode {
    BlockNode::Paragraph { content: vec![plain(text)] }
}

#[test]
fn content_that_already_fits_is_rendered_at_full_scale() {
    let blocks = vec![paragraph("Short.")];
    let mut fs = test_font_system();
    let (geometry, diagrams, base_stylesheet) = (geometry(), DiagramTable::new(), Stylesheet::default());
    let output = layout_slide_with_shrink(&blocks, &mut fs, &deck_context(&geometry, &diagrams, &base_stylesheet), &SlideLayoutStyle::default(), 1);
    assert_eq!(output.pages.len(), 1);
    let has_full_size_text =
        output.pages[0].elements.iter().any(|e| matches!(e, sardown_layout::PositionedElement::TextRun { size, .. } if (*size - 12.0).abs() < 0.01));
    assert!(has_full_size_text, "expected the untouched body_size_pt (12.0) to survive at scale 1.0");
}

#[test]
fn content_that_overflows_at_full_scale_shrinks_until_it_fits() {
    // Each paragraph is short enough to never wrap onto a second line at *any* scale tried here
    // (well under the ~255pt content width even at the full 12pt size), so its rendered height is
    // exactly `estimate_line_height(body_size_pt * scale)` (`size*1.4 + LINE_SPACING_PT`) plus one
    // more `LINE_SPACING_PT` from `layout_impl`'s own per-block loop -- i.e. `size*1.4 + 8.0` pt
    // per paragraph, with no line-wrapping math to guess at.
    //
    // Usable height on this 60mm-tall, 5mm-margin page is
    // `60 * 2.834_645_7 - 2 * (5 * 2.834_645_7)` ~= 141.7pt. With 8 paragraphs:
    //   scale 1.0: 8 * (12.0*1.4 + 8.0)  = 8 * 24.8 = 198.4pt  (overflows 141.7pt)
    //   scale 0.5: 8 * (6.0*1.4  + 8.0)  = 8 * 16.4 = 131.2pt  (fits within 141.7pt, ~10.5pt to spare)
    // so the shrink loop is guaranteed to find *some* fitting scale at or before the 0.5 floor.
    let blocks: Vec<BlockNode> = (0..8).map(|_| paragraph("Body text line.")).collect();
    let mut fs = test_font_system();
    let (geometry, diagrams, base_stylesheet) = (geometry(), DiagramTable::new(), Stylesheet::default());
    let output = layout_slide_with_shrink(&blocks, &mut fs, &deck_context(&geometry, &diagrams, &base_stylesheet), &SlideLayoutStyle::default(), 1);
    assert_eq!(output.pages.len(), 1, "expected the shrink loop to find a fitting scale");
    let smallest_size = output.pages[0]
        .elements
        .iter()
        .filter_map(|e| match e {
            sardown_layout::PositionedElement::TextRun { size, .. } => Some(*size),
            _ => None,
        })
        .fold(f32::MAX, f32::min);
    assert!(smallest_size < 12.0, "expected text to have shrunk below the base 12pt size, got {smallest_size}");
}

#[test]
fn content_that_still_overflows_at_the_floor_scale_renders_every_page_without_dropping_content() {
    // Same short, non-wrapping paragraph as above, but enough of them (30) that even at the 0.5
    // floor scale the total height (30 * 16.4 = 492pt) is far beyond the ~141.7pt usable height --
    // comfortably overflowing with no risk of a near-miss on either side.
    let blocks: Vec<BlockNode> = (0..30).map(|_| paragraph("Body text line.")).collect();
    let mut fs = test_font_system();
    let (geometry, diagrams, base_stylesheet) = (geometry(), DiagramTable::new(), Stylesheet::default());
    let output = layout_slide_with_shrink(&blocks, &mut fs, &deck_context(&geometry, &diagrams, &base_stylesheet), &SlideLayoutStyle::default(), 1);
    assert!(output.pages.len() > 1, "expected genuine overflow content to span more than one page rather than being dropped");
}
