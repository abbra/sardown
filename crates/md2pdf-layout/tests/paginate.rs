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

fn path_y_bounds(points: &[md2pdf_layout::PathCommand]) -> (f32, f32) {
    let ys: Vec<f32> = points
        .iter()
        .filter_map(|p| match *p {
            md2pdf_layout::PathCommand::MoveTo(_, y) | md2pdf_layout::PathCommand::LineTo(_, y) => Some(y),
            _ => None,
        })
        .collect();
    (ys.iter().cloned().fold(f32::MAX, f32::min), ys.iter().cloned().fold(f32::MIN, f32::max))
}

#[test]
fn code_block_background_fully_encloses_the_first_lines_ascender() {
    // `TextRun::y` is a baseline, not a glyph top, so a background sized as `baseline_y..end_y`
    // (a small flat pad instead of one accounting for ascent) leaves the tops of ascenders (e.g.
    // "T", "P") poking out above the box — this pins the padding so it can't regress back to that.
    let ast = vec![BlockNode::CodeBlock { language: None, tokens: vec![HighlightedToken { text: "Test\n".to_string(), color: [0, 0, 0] }] }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;

    let first_line_baseline_y = pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { y, .. } => Some(*y),
            _ => None,
        })
        .expect("expected at least one text run");
    let (bg_top, _) = pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::Path { points, fill: Some(_), .. } => Some(path_y_bounds(points)),
            _ => None,
        })
        .expect("expected a background path");

    assert!(
        first_line_baseline_y - bg_top >= 6.0,
        "background top ({bg_top}) is not far enough above the first line's baseline ({first_line_baseline_y}) \
         to cover its ascender"
    );
}

#[test]
fn code_block_spanning_pages_draws_a_background_on_each_page_it_touches() {
    // Enough lines that the block cannot fit on one page, forcing `place_inline_content` to
    // break mid-block. Regression test for a bug where the background rectangle used `start_y`
    // from the first page and `end_y` from the last page in one rect, drawing a stray band on
    // the continuation page unrelated to any of the block's actual text.
    let long_code: String = (0..80).map(|i| format!("line {i}\n")).collect();
    let ast = vec![BlockNode::CodeBlock { language: None, tokens: vec![HighlightedToken { text: long_code, color: [0, 0, 0] }] }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    assert!(pages.len() >= 2, "expected the code block to span at least 2 pages, got {}", pages.len());

    for page in &pages {
        let backgrounds: Vec<_> = page
            .elements
            .iter()
            .filter_map(|e| match e {
                PositionedElement::Path { points, fill: Some(_), .. } => Some(path_y_bounds(points)),
                _ => None,
            })
            .collect();
        assert_eq!(backgrounds.len(), 1, "expected exactly one code-block background on each page it touches");

        let text_ys: Vec<f32> = page
            .elements
            .iter()
            .filter_map(|e| match e {
                PositionedElement::TextRun { y, .. } => Some(*y),
                _ => None,
            })
            .collect();
        assert!(!text_ys.is_empty(), "expected text runs on every page the code block touches");
        let min_text_y = text_ys.iter().cloned().fold(f32::MAX, f32::min);
        let max_text_y = text_ys.iter().cloned().fold(f32::MIN, f32::max);
        let (bg_top, bg_bottom) = backgrounds[0];

        // The background must bracket THIS page's own text, not a stale range left over from a
        // different page (the bug this test guards against).
        assert!(bg_top <= min_text_y, "background top ({bg_top}) does not cover this page's topmost text ({min_text_y})");
        assert!(bg_bottom >= max_text_y - 1.0, "background bottom ({bg_bottom}) does not cover this page's bottommost text ({max_text_y})");
    }
}

use md2pdf_ast::ColumnAlignment;

fn cell(text: &str) -> Vec<InlineNode> {
    vec![plain_inline(text)]
}

#[test]
fn table_row_grows_to_fit_a_wrapped_multiline_cell() {
    // Regression test: row height used to be a fixed 20pt regardless of content, so a cell whose
    // text wrapped to multiple lines (common with real-world "Description"-style columns)
    // overlapped the row below it instead of pushing it down.
    // Column A's own content is a single short word in every row: against a hugely disparate
    // column B (one long, wrapping cell), `column_widths`'s proportional distribution squeezes
    // column A down toward its floor width — multi-word column-A content (even "Column Alpha")
    // could itself wrap there, which is correct behavior for cosmic-text (`LayoutRun::text` is
    // documented as the pre-wrap source line, so a wrapped multi-word cell legitimately produces
    // more than one line) but is a separate concern from what this test isolates: single-word
    // cells never wrap regardless of how narrow the floor width gets.
    let headers = vec![cell("A"), cell("Col B")];
    let rows = vec![
        vec![
            cell("x"),
            // Long enough to wrap onto at least 5 lines: 2 lines' worth of intra-cell spacing
            // (16.8pt each) alone would exceed the old fixed 20pt row height, so this reliably
            // reproduces the original overlap bug if the row-height fix regresses.
            cell(&"a very long word ".repeat(12)),
        ],
        vec![cell("x"), cell("y")],
    ];
    let ast = vec![BlockNode::Table { headers, rows, alignments: vec![ColumnAlignment::None; 2] }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;

    let mut text_ys: Vec<f32> = pages[0]
        .elements
        .iter()
        .filter_map(|e| match e {
            PositionedElement::TextRun { y, .. } => Some(*y),
            _ => None,
        })
        .collect();
    text_ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    text_ys.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    // header line + several wrapped lines for row 1 + one line for row 2: definitely more than
    // the 3 lines a fixed-20pt-row layout would have produced.
    assert!(text_ys.len() > 3, "expected multiple distinct line positions from wrapping, got {}", text_ys.len());

    // Gaps are NOT expected to be uniform: cosmic-text spaces wrapped continuation lines within
    // one `place_inline_content` call using its own tighter internal line-height (`size * 1.4`),
    // while separate calls (row-to-row, header-to-row) are spaced using this crate's own
    // `estimate_line_height` (`size * 1.4 + LINE_SPACING_PT`, larger). What actually matters —
    // and what the fixed-20pt-row bug broke — is that every line strictly follows the previous
    // one; a bug that let row 2 start inside row 1's wrapped content would show up as a
    // zero-or-negative gap here.
    let gaps: Vec<f32> = text_ys.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(gaps.iter().all(|&g| g > 0.0), "expected every line to be strictly below the previous one, gaps: {gaps:?}");
}

#[test]
fn table_cell_wraps_within_its_own_column_not_the_remaining_page_width() {
    // Regression test: cell wrapping used to be capped at "remaining width to the right margin"
    // instead of the cell's own column width, letting long text in one column visually run into
    // the next column's space before ever wrapping.
    let headers = vec![cell("A"), cell("B")];
    let rows = vec![vec![cell("x"), cell("a very long cell value that must wrap across several lines given a narrow column width")]];
    let ast = vec![BlockNode::Table { headers, rows, alignments: vec![ColumnAlignment::None; 2] }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;

    let text_runs: Vec<_> = pages[0].elements.iter().filter(|e| matches!(e, PositionedElement::TextRun { .. })).collect();
    let wrapped_lines_for_long_cell = text_runs.iter().filter(|e| matches!(e, PositionedElement::TextRun { y, .. } if *y > 20.0)).count();
    assert!(wrapped_lines_for_long_cell > 1, "expected the long cell's text to wrap onto multiple lines within its own column");
}

#[test]
fn table_row_never_splits_across_a_page_break() {
    // Regression test: tables used to have no page-break awareness at all, so a row starting
    // near the bottom of a page could have some of its cells land on the next page while others
    // stayed behind -- each cell resets to a `row_top_y` captured on whichever page it happened
    // to render on, scattering the row's remaining cells across an unrelated part of the wrong
    // page instead of keeping the whole row together.
    let headers = vec![cell("A"), cell("B")];
    let rows: Vec<Vec<Vec<InlineNode>>> = (0..40).map(|i| vec![cell(&format!("row{i}-left")), cell(&format!("row{i}-right"))]).collect();
    let ast = vec![BlockNode::Table { headers, rows, alignments: vec![ColumnAlignment::None; 2] }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    assert!(pages.len() >= 2, "expected the table to span at least 2 pages, got {}", pages.len());

    for i in 0..40 {
        let left_marker = format!("row{i}-left");
        let right_marker = format!("row{i}-right");
        let page_of = |marker: &str| {
            pages
                .iter()
                .position(|page| page.elements.iter().any(|e| matches!(e, PositionedElement::TextRun { text, .. } if text == marker)))
                .unwrap_or_else(|| panic!("marker {marker:?} not found on any page"))
        };
        assert_eq!(page_of(&left_marker), page_of(&right_marker), "row {i}'s two cells landed on different pages");
    }
}

#[test]
fn table_header_separator_line_sits_between_rows_not_through_row_ones_text() {
    // Regression test: the header/row-1 separator line used to be positioned at row 1's own
    // baseline (`cursor.y` after placing a row is the *next* row's baseline, not a safe gap
    // boundary), drawing it straight through the middle of row 1's text instead of in the empty
    // gap between the header and row 1.
    let headers = vec![cell("Layer"), cell("Bytes")];
    let rows = vec![vec![cell("Outer SignedData"), cell("~555 B")]];
    let ast = vec![BlockNode::Table { headers, rows, alignments: vec![ColumnAlignment::None; 2] }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;

    let row1_baseline_y = pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { text, y, .. } if text == "Outer SignedData" => Some(*y),
            _ => None,
        })
        .expect("expected row 1's text run");

    // Approximate row 1's visual glyph extent as [baseline - ascent, baseline + descent].
    let row1_top = row1_baseline_y - 12.0 * 0.8;
    let row1_bottom = row1_baseline_y + 12.0 * 0.2;

    let grid = pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::Path { points, fill: None, .. } => Some(points),
            _ => None,
        })
        .expect("expected a grid Path");

    // A horizontal segment is a MoveTo/LineTo pair sharing the same y; none should fall inside
    // row 1's visual text extent.
    for window in grid.windows(2) {
        if let (md2pdf_layout::PathCommand::MoveTo(_, y1), md2pdf_layout::PathCommand::LineTo(_, y2)) = (&window[0], &window[1]) {
            if (y1 - y2).abs() < 0.01 {
                assert!(*y1 < row1_top || *y1 > row1_bottom, "a horizontal grid line at y={y1} falls inside row 1's text extent [{row1_top}, {row1_bottom}]");
            }
        }
    }
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

#[test]
fn mermaid_diagram_taller_than_a_full_page_is_scaled_down_to_fit_one_page() {
    // Regression test: a diagram was only ever scaled down by width, never by height, so one
    // taller than an entire page's content area (not just "the remaining space on the current
    // page") still overflowed past the bottom margin even on a fresh page -- breaking to a new
    // page couldn't help, since the diagram was too big for ANY page, not just this one.
    let ast = vec![BlockNode::MermaidDiagram { id: "d1".to_string(), source: "flowchart TD\n A-->B".to_string() }];
    let mut diagrams = DiagramTable::new();
    // Far taller (aspect-wise) than a US Letter page's content area at 1in margins (~648pt).
    diagrams.insert("d1".to_string(), CompiledDiagram { svg: "<svg/>".to_string(), width: 100.0, height: 2000.0 });

    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &diagrams).pages;

    assert_eq!(pages.len(), 1, "a too-tall diagram should be scaled to fit one page, not overflow onto extra pages");
    match &pages[0].elements[0] {
        PositionedElement::VectorGraphic { width, height, .. } => {
            assert!(*height <= 648.5, "diagram height ({height}) exceeds a full page's content height");
            assert!(*width > 0.0 && *width < 100.0, "expected the diagram to be scaled down proportionally (preserving aspect ratio), got width={width}");
        }
        other => panic!("expected VectorGraphic, got {other:?}"),
    }
}

#[test]
fn heading_after_mermaid_diagram_does_not_overlap_the_diagrams_bottom_edge() {
    // Regression test: a diagram has a crisp, hard bottom edge (unlike wrapped body text, where
    // consecutive baselines sitting close together is normal typography). The flat
    // LINE_SPACING_PT gap the layout loop adds after every block isn't enough clearance for the
    // next block's own ascender, so a heading placed right after a diagram had its glyphs poke
    // up above their own baseline straight into the diagram's box -- visibly overlapping it.
    // HEADING_SIZES[1] (level 2) is 22.0 -- built by hand here (not via `plain_inline`, which
    // always uses body-text size 12.0) so estimate_next_block_ascent_pt sees the same size a
    // real level-2 heading parsed from Markdown would carry.
    let heading_size = 22.0;
    let heading_content = InlineNode {
        text: "Next".to_string(),
        style: TextStyle { bold: false, italic: false, size: heading_size, color: [0, 0, 0] },
        link_target: None,
    };
    let ast = vec![
        BlockNode::MermaidDiagram { id: "d1".to_string(), source: "flowchart TD\n A-->B".to_string() },
        BlockNode::Heading { level: 2, id: "next".to_string(), content: vec![heading_content] },
    ];
    let mut diagrams = DiagramTable::new();
    diagrams.insert("d1".to_string(), CompiledDiagram { svg: "<svg/>".to_string(), width: 140.0, height: 278.0 });

    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &diagrams).pages;

    let diagram_bottom = pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::VectorGraphic { y, height, .. } => Some(y + height),
            _ => None,
        })
        .expect("expected a VectorGraphic element");

    let heading_baseline = pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { y, .. } => Some(*y),
            _ => None,
        })
        .expect("expected a heading TextRun");

    // H2's approximate ascent is size*0.8 (matches estimate_next_block_ascent_pt's own
    // approximation) -- the heading's visual top must sit at or below the diagram's bottom edge.
    let heading_visual_top = heading_baseline - heading_size * 0.8;
    assert!(heading_visual_top >= diagram_bottom, "heading's ascender (top={heading_visual_top}) overlaps the diagram's bottom edge ({diagram_bottom})");
}

#[test]
fn page_break_forces_content_onto_a_new_page() {
    let ast = vec![
        BlockNode::Paragraph { content: vec![plain_inline("first")] },
        BlockNode::PageBreak,
        BlockNode::Paragraph { content: vec![plain_inline("second")] },
    ];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    assert_eq!(pages.len(), 2, "expected PageBreak to force a second page");

    let page0_has_first = pages[0].elements.iter().any(|e| matches!(e, PositionedElement::TextRun { text, .. } if text == "first"));
    let page1_has_second = pages[1].elements.iter().any(|e| matches!(e, PositionedElement::TextRun { text, .. } if text == "second"));
    assert!(page0_has_first, "expected 'first' to stay on page 0");
    assert!(page1_has_second, "expected 'second' to be pushed onto page 1 by the break");
}

#[test]
fn page_break_at_the_very_start_does_not_create_a_blank_leading_page() {
    let ast = vec![BlockNode::PageBreak, BlockNode::Paragraph { content: vec![plain_inline("only")] }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    assert_eq!(pages.len(), 1, "a PageBreak with nothing yet on the current page should not force a blank page");
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
