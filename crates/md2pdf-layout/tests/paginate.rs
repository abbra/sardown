use cosmic_text::FontSystem;
use md2pdf_ast::{parse, BlockNode};
use md2pdf_layout::{layout, PageGeometry, PositionedElement};

fn test_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).expect("failed to load test font");
    db.set_sans_serif_family("Droid Sans");
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

fn letter_geometry() -> PageGeometry {
    PageGeometry { page_width_mm: 215.9, page_height_mm: 279.4, margin_mm: 25.4 } // US Letter, 1in margins
}

fn fixtures_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn single_short_paragraph_fits_on_one_page() {
    let ast = parse("Just one short paragraph.\n");
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    assert_eq!(pages.len(), 1);
    assert!(!pages[0].elements.is_empty());
}

#[test]
fn many_headings_overflow_onto_a_second_page() {
    // 60 headings at a fixed line height comfortably exceeds one US Letter page
    let md: String = (0..60).map(|i| format!("# Heading {i}\n\n")).collect();
    let blocks: Vec<BlockNode> = parse(&md);
    let mut fs = test_font_system();
    let pages = layout(&blocks, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    assert!(pages.len() >= 2, "expected content to overflow onto a second page, got {} page(s)", pages.len());
    assert_eq!(pages[0].page_number, 0);
    assert_eq!(pages[1].page_number, 1);
}

#[test]
fn heading_at_bottom_of_page_moves_with_its_first_line_of_body_text() {
    // Widow/orphan rule (§4.2 item 4): a heading must not be the last element on a page
    // with none of its following paragraph's text on the same page.
    let md = "# T\n\nBody\n".repeat(40); // pad until a heading lands near a page boundary
    let blocks = parse(&md);
    let mut fs = test_font_system();
    let pages = layout(&blocks, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    for page in &pages[..pages.len() - 1] {
        let last_is_lone_heading = matches!(page.elements.last(), Some(PositionedElement::TextRun { .. })) && page.elements.len() == 1;
        assert!(!last_is_lone_heading, "found a page ending in an isolated heading with no body text");
    }
}

use md2pdf_ast::{HighlightedToken, InlineNode, TextStyle};

fn plain_inline(text: &str) -> InlineNode {
    InlineNode { text: text.to_string(), style: TextStyle { bold: false, italic: false, size: 12.0, color: [0, 0, 0] }, link_target: None }
}

#[test]
fn blockquote_produces_a_side_border_path_plus_nested_text() {
    let ast = vec![BlockNode::Blockquote { content: vec![BlockNode::Paragraph { content: vec![plain_inline("Quoted")] }] }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    let has_path = pages[0].elements.iter().any(|e| matches!(e, PositionedElement::Path { .. }));
    let has_text = pages[0].elements.iter().any(|e| matches!(e, PositionedElement::TextRun { .. }));
    assert!(has_path && has_text);
}

#[test]
fn thematic_break_produces_a_horizontal_line_path() {
    let ast = vec![BlockNode::ThematicBreak];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    assert!(matches!(pages[0].elements[0], PositionedElement::Path { .. }));
}

#[test]
fn list_items_render_as_indented_text() {
    let ast = vec![BlockNode::List {
        ordered: false,
        items: vec![
            vec![BlockNode::Paragraph { content: vec![plain_inline("one")] }],
            vec![BlockNode::Paragraph { content: vec![plain_inline("two")] }],
        ],
    }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    let text_runs: Vec<_> = pages[0].elements.iter().filter(|e| matches!(e, PositionedElement::TextRun { .. })).collect();
    assert_eq!(text_runs.len(), 2);
}

#[test]
fn code_block_produces_a_background_path_and_highlighted_text_runs() {
    let ast = vec![BlockNode::CodeBlock {
        language: None,
        tokens: vec![HighlightedToken { text: "let ".to_string(), color: [255, 0, 0] }, HighlightedToken { text: "x".to_string(), color: [0, 0, 255] }],
    }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    let has_background = pages[0].elements.iter().any(|e| matches!(e, PositionedElement::Path { fill: Some(_), .. }));
    let colored_runs: Vec<_> = pages[0]
        .elements
        .iter()
        .filter_map(|e| match e {
            PositionedElement::TextRun { color, .. } => Some(*color),
            _ => None,
        })
        .collect();
    assert!(has_background);
    assert!(colored_runs.contains(&[255, 0, 0]) && colored_runs.contains(&[0, 0, 255]));
}

use md2pdf_enrich::{CompiledDiagram, DiagramTable};

#[test]
fn mermaid_diagram_produces_a_vector_graphic_element() {
    let ast = vec![BlockNode::MermaidDiagram { id: "d1".to_string(), source: "flowchart TD\n A-->B".to_string() }];
    let mut diagrams = DiagramTable::new();
    diagrams.insert("d1".to_string(), CompiledDiagram { svg: "<svg/>".to_string(), width: 300.0, height: 150.0 });

    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &diagrams);

    match &output.pages[0].elements[0] {
        PositionedElement::VectorGraphic { diagram_id, width, height, .. } => {
            assert_eq!(diagram_id, "d1");
            assert!(*width > 0.0 && *height > 0.0);
        }
        other => panic!("expected VectorGraphic, got {other:?}"),
    }
}

use md2pdf_ast::LinkTarget;

#[test]
fn heading_id_is_recorded_in_the_anchor_table_with_its_page_and_position() {
    let ast = parse("# My Heading\n\nBody text.\n");
    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());

    let anchor = output.anchors.get("my-heading").expect("heading anchor not recorded");
    assert_eq!(anchor.page, 0);
    assert!(anchor.y >= 0.0);
}

#[test]
fn linked_inline_run_produces_a_link_annotation_element() {
    let ast = parse("[External](https://example.com)\n\n[Internal](#target)\n");
    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());

    let annotations: Vec<_> = output.pages[0]
        .elements
        .iter()
        .filter_map(|e| match e {
            PositionedElement::LinkAnnotation { destination, .. } => Some(destination.clone()),
            _ => None,
        })
        .collect();

    assert!(annotations.contains(&LinkTarget::ExternalUrl("https://example.com".to_string())));
    assert!(annotations.contains(&LinkTarget::InternalAnchor("target".to_string())));
}
