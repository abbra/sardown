use cosmic_text::FontSystem;
use sardown_ast::{parse, BlockNode};
use sardown_layout::{layout, PageGeometry, PositionedElement};

fn test_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/DroidSans.ttf")).expect("failed to load test font");
    db.set_sans_serif_family("Droid Sans");
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

fn letter_geometry() -> PageGeometry {
    PageGeometry { page_width_mm: 215.9, page_height_mm: 279.4, margin_mm: 25.4, ..Default::default() }
    // US Letter, 1in margins
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

use sardown_ast::{HighlightedToken, InlineNode, TextStyle};

fn plain_inline(text: &str) -> InlineNode {
    InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size: 12.0, color: [0, 0, 0], font_family: "sans-serif".to_string() },
        link_target: None,
    }
}

fn sized_inline(text: &str, size: f32) -> InlineNode {
    InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size, color: [0, 0, 0], font_family: "sans-serif".to_string() },
        link_target: None,
    }
}

#[test]
fn heading_has_more_space_before_it_than_after_it() {
    // Regression test: with no extra "space before a heading," a new section's title sat right
    // up against the *previous* section's last line while the gap to its OWN following content
    // stayed the same small inter-block spacing -- backwards from normal document typography,
    // making the heading read as the tail of the wrong section.
    let ast = vec![
        BlockNode::Paragraph { content: vec![plain_inline("End of section one.")] },
        BlockNode::Heading { level: 2, id: "two".to_string(), content: vec![sized_inline("Section Two", 22.0)] },
        BlockNode::Paragraph { content: vec![plain_inline("Start of section two.")] },
    ];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;

    let y_of = |text: &str| {
        pages[0]
            .elements
            .iter()
            .find_map(|e| match e {
                PositionedElement::TextRun { y, text: t, .. } if t == text => Some(*y),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing text run {text:?}"))
    };

    let end_of_p1 = y_of("End of section one.");
    let heading_y = y_of("Section Two");
    let start_of_p2 = y_of("Start of section two.");

    let gap_before_heading = heading_y - end_of_p1;
    let gap_after_heading = start_of_p2 - heading_y;

    assert!(gap_before_heading > gap_after_heading, "expected more space before a heading ({gap_before_heading}) than after it ({gap_after_heading})");
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
fn blockquote_border_does_not_overrun_into_the_following_blocks_ascender() {
    // Regression test: the border's start_y was the first line's *baseline* (missing its
    // ascender entirely), and its end_y was the cursor position *after* the last line's full
    // line height -- which already includes the gap reserved for whatever block comes next. On
    // real documents this made the border start visibly too low and run down into the following
    // paragraph's own text.
    let ast = vec![
        BlockNode::Blockquote { content: vec![BlockNode::Paragraph { content: vec![plain_inline("Quoted")] }] },
        BlockNode::Paragraph { content: vec![plain_inline("Following")] },
    ];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;

    let quoted_baseline = pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { y, text, .. } if text == "Quoted" => Some(*y),
            _ => None,
        })
        .expect("expected the quoted text run");
    let following_baseline = pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { y, text, .. } if text == "Following" => Some(*y),
            _ => None,
        })
        .expect("expected the following text run");
    let (border_top, border_bottom) = pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::Path { points, stroke: Some(_), .. } => match points.as_slice() {
                [sardown_layout::PathCommand::MoveTo(_, y0), sardown_layout::PathCommand::LineTo(_, y1)] => Some((*y0, *y1)),
                _ => None,
            },
            _ => None,
        })
        .expect("expected the blockquote border path");

    // 12pt body text's ascent is ~9.6pt (size*0.8, matching estimate_next_block_ascent_pt) -- the
    // border's top must reach up to at least the quoted text's visual top, not sit at its baseline.
    assert!(border_top <= quoted_baseline - 8.0, "border top ({border_top}) doesn't reach the quoted text's ascender (baseline={quoted_baseline})");
    // The border's bottom must stay clear of the following paragraph's own ascender (~9.6pt above
    // its baseline), not run down into it.
    assert!(border_bottom <= following_baseline - 8.0, "border bottom ({border_bottom}) overruns into the following paragraph (baseline={following_baseline})");
}

#[test]
fn blockquote_border_spanning_pages_draws_a_segment_on_each_page_it_touches() {
    // Regression test: a blockquote long enough to spill onto a second page had its border drawn
    // as ONE path using start_y (captured on the first page) and end_y (captured on the second
    // page) -- two different pages' coordinate systems combined into a single line, pushed onto
    // whichever page happened to be current at the end. On a real document this produced a huge,
    // meaningless vertical line spanning almost the entire continuation page, cutting through
    // unrelated headings and paragraphs that came after the blockquote -- same category of bug
    // the code block background and table grid already had to handle for page-spanning content.
    let long_text: String = "word ".repeat(2000); // long enough to force a page break mid-paragraph
    let ast = vec![
        BlockNode::Blockquote { content: vec![BlockNode::Paragraph { content: vec![plain_inline(&long_text)] }] },
        BlockNode::Heading { level: 2, id: "after".to_string(), content: vec![sized_inline("After", 22.0)] },
    ];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    assert!(pages.len() >= 2, "expected the blockquote to span at least 2 pages, got {}", pages.len());

    // A single page's usable content height (page height minus top+bottom margins, plus some
    // slack for the ascent padding subtracted from the top) at 1in margins on US Letter -- no
    // border segment should ever exceed this by much, since that would mean it incorrectly
    // combined y-coordinates from two different pages (which would overshoot by hundreds of
    // points, not a handful).
    const MAX_PAGE_CONTENT_HEIGHT_PT: f32 = 700.0;

    let mut total_segments = 0;
    for page in &pages {
        for element in &page.elements {
            if let PositionedElement::Path { points, stroke: Some(_), .. } = element {
                if let [sardown_layout::PathCommand::MoveTo(_, y0), sardown_layout::PathCommand::LineTo(_, y1)] = points.as_slice() {
                    total_segments += 1;
                    let span = (y1 - y0).abs();
                    assert!(
                        span <= MAX_PAGE_CONTENT_HEIGHT_PT,
                        "border segment on page {} spans {span}pt, more than a single page's content height -- \
                         it likely combined coordinates from two different pages",
                        page.page_number
                    );
                }
            }
        }
    }
    assert!(total_segments >= 2, "expected at least one border segment per page the blockquote touches, got {total_segments}");
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
        start: None,
        items: vec![vec![BlockNode::Paragraph { content: vec![plain_inline("one")] }], vec![BlockNode::Paragraph { content: vec![plain_inline("two")] }]],
    }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    // Each item's own text and its bullet marker are shaped as separate spans (one TextRun per
    // source InlineNode, even sharing the same style) -- 2 items x (1 marker + 1 text) = 4.
    let text_runs: Vec<_> = pages[0].elements.iter().filter(|e| matches!(e, PositionedElement::TextRun { .. })).collect();
    assert_eq!(text_runs.len(), 4);
}

fn list_line_texts(pages: &[sardown_layout::PositionedPage]) -> Vec<String> {
    pages[0]
        .elements
        .iter()
        .filter_map(|e| match e {
            PositionedElement::TextRun { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn unordered_list_items_get_a_bullet_marker() {
    let ast = vec![BlockNode::List {
        ordered: false,
        start: None,
        items: vec![vec![BlockNode::Paragraph { content: vec![plain_inline("one")] }], vec![BlockNode::Paragraph { content: vec![plain_inline("two")] }]],
    }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    let texts = list_line_texts(&pages);
    assert!(texts.iter().any(|t| t.contains('\u{2022}') && t.contains("one")), "expected a bullet before the first item, got: {texts:?}");
    assert!(texts.iter().any(|t| t.contains('\u{2022}') && t.contains("two")), "expected a bullet before the second item, got: {texts:?}");
}

#[test]
fn ordered_list_items_are_numbered_sequentially() {
    let ast = vec![BlockNode::List {
        ordered: true,
        start: Some(1),
        items: vec![
            vec![BlockNode::Paragraph { content: vec![plain_inline("first")] }],
            vec![BlockNode::Paragraph { content: vec![plain_inline("second")] }],
            vec![BlockNode::Paragraph { content: vec![plain_inline("third")] }],
        ],
    }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    let texts = list_line_texts(&pages);
    assert!(texts.iter().any(|t| t.contains("1.") && t.contains("first")), "got: {texts:?}");
    assert!(texts.iter().any(|t| t.contains("2.") && t.contains("second")), "got: {texts:?}");
    assert!(texts.iter().any(|t| t.contains("3.") && t.contains("third")), "got: {texts:?}");
}

#[test]
fn ordered_list_honors_a_non_default_start_number() {
    let ast = vec![BlockNode::List {
        ordered: true,
        start: Some(5),
        items: vec![vec![BlockNode::Paragraph { content: vec![plain_inline("fifth")] }], vec![BlockNode::Paragraph { content: vec![plain_inline("sixth")] }]],
    }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    let texts = list_line_texts(&pages);
    assert!(texts.iter().any(|t| t.contains("5.") && t.contains("fifth")), "got: {texts:?}");
    assert!(texts.iter().any(|t| t.contains("6.") && t.contains("sixth")), "got: {texts:?}");
}

#[test]
fn a_list_item_whose_first_child_is_not_a_paragraph_is_left_without_a_marker() {
    // Documented v1 limitation: a marker is only prepended when the item's first child is a
    // Paragraph (the overwhelming common case for real-world Markdown lists). An item starting
    // directly with a nested list has no natural place to attach one without extra machinery --
    // it must not panic or drop the nested list's own content either.
    let ast = vec![BlockNode::List {
        ordered: false,
        start: None,
        items: vec![vec![BlockNode::List { ordered: false, start: None, items: vec![vec![BlockNode::Paragraph { content: vec![plain_inline("nested")] }]] }]],
    }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    let texts = list_line_texts(&pages);
    assert!(texts.iter().any(|t| t.contains("nested")), "expected the nested item's own text to still render, got: {texts:?}");
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

fn path_y_bounds(points: &[sardown_layout::PathCommand]) -> (f32, f32) {
    let ys: Vec<f32> = points
        .iter()
        .filter_map(|p| match *p {
            sardown_layout::PathCommand::MoveTo(_, y) | sardown_layout::PathCommand::LineTo(_, y) => Some(y),
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
fn code_block_background_does_not_overshoot_past_the_last_line_by_a_whole_extra_line() {
    // Regression test: `cursor.y` after placing the code block's content is the position where a
    // *next* line would start (baseline + full line height, which already includes the trailing
    // inter-block gap) -- not the last line's own visual bottom. Using it directly (plus a small
    // fixed pad) made the background extend a whole extra code line's height past the actual
    // last line, bleeding into whatever content came after.
    let ast = vec![BlockNode::CodeBlock { language: None, tokens: vec![HighlightedToken { text: "one\ntwo\n".to_string(), color: [0, 0, 0] }] }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;

    let mut baselines: Vec<f32> = pages[0]
        .elements
        .iter()
        .filter_map(|e| match e {
            PositionedElement::TextRun { y, .. } => Some(*y),
            _ => None,
        })
        .collect();
    baselines.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let last_line_baseline = *baselines.last().expect("expected at least one text run");

    let (_, bg_bottom) = pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::Path { points, fill: Some(_), .. } => Some(path_y_bounds(points)),
            _ => None,
        })
        .expect("expected a background path");

    // A code line's descender needs only a few points of clearance below its own baseline --
    // nowhere near a whole extra line height (~18pt at this 10pt code font size).
    assert!(
        bg_bottom - last_line_baseline < 10.0,
        "background bottom ({bg_bottom}) overshoots the last line's baseline ({last_line_baseline}) by \
         {}pt -- looks like it included a whole extra line's height",
        bg_bottom - last_line_baseline
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

use sardown_ast::ColumnAlignment;

fn cell(text: &str) -> Vec<InlineNode> {
    vec![plain_inline(text)]
}

#[test]
fn table_cell_text_has_horizontal_padding_from_the_column_edges() {
    // Regression test: a cell's text started exactly at the column's left grid line (x =
    // margin_pt for the first column, since it has no preceding column) with zero padding,
    // visually gluing it to the vertical divider between columns.
    let headers = vec![cell("A"), cell("B")];
    let rows = vec![vec![cell("x"), cell("y")]];
    let ast = vec![BlockNode::Table { headers, rows, alignments: vec![ColumnAlignment::None; 2] }];
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;

    let margin_pt = 25.4 * 2.834_645_7; // matches Cursor's own mm-to-pt conversion
    let first_col_text_x = pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { x, text, .. } if text == "A" => Some(*x),
            _ => None,
        })
        .expect("expected the first column's header text");

    assert!(
        first_col_text_x > margin_pt + 2.0,
        "expected cell text ({first_col_text_x}) to start with some padding after the column's left edge ({margin_pt}), not flush against it"
    );
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
        if let (sardown_layout::PathCommand::MoveTo(_, y1), sardown_layout::PathCommand::LineTo(_, y2)) = (&window[0], &window[1]) {
            if (y1 - y2).abs() < 0.01 {
                assert!(*y1 < row1_top || *y1 > row1_bottom, "a horizontal grid line at y={y1} falls inside row 1's text extent [{row1_top}, {row1_bottom}]");
            }
        }
    }
}

use sardown_enrich::{CompiledDiagram, DiagramTable};

#[test]
fn mermaid_diagram_produces_a_vector_graphic_element() {
    let ast = vec![BlockNode::MermaidDiagram { id: "d1".to_string(), source: "flowchart TD\n A-->B".to_string(), line: 1, column: 1, file: None }];
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
    let ast = vec![BlockNode::MermaidDiagram { id: "d1".to_string(), source: "flowchart TD\n A-->B".to_string(), line: 1, column: 1, file: None }];
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
fn a_base64_data_uri_image_produces_a_raster_image_element_with_no_base_dir_needed() {
    let uri = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAGUlEQVR4AQEOAPH/AP8AAP8AAAD/AAD/AAAf7gP9ii433QAAAABJRU5ErkJggg==";
    let ast = vec![BlockNode::Image { alt: "test".to_string(), title: None, source: sardown_ast::ImageSource::DataUri(uri.to_string()) }];
    let mut fs = test_font_system();
    // A deliberately nonexistent base_dir: a data URI is fully self-contained and must not need
    // any filesystem access to render.
    let output = layout(&ast, &letter_geometry(), &mut fs, std::path::Path::new("/nonexistent"), &DiagramTable::new());

    match &output.pages[0].elements[0] {
        PositionedElement::RasterImage { image_id, width, height, .. } => {
            assert_eq!(image_id, uri);
            assert!(*width > 0.0 && *height > 0.0);
        }
        other => panic!("expected RasterImage, got {other:?}"),
    }
}

#[test]
fn an_embedded_svg_image_produces_a_vector_graphic_element() {
    let ast = vec![BlockNode::Image {
        alt: "test".to_string(),
        title: None,
        source: sardown_ast::ImageSource::Embedded(std::path::PathBuf::from("test-vector.svg")),
    }];
    let mut fs = test_font_system();
    // No pre-populated DiagramTable -- layout_impl must discover and merge the SVG file itself,
    // the same way it discovers embedded raster images via decode_images.
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());

    match &output.pages[0].elements[0] {
        PositionedElement::VectorGraphic { diagram_id, width, height, .. } => {
            assert_eq!(diagram_id, "test-vector.svg");
            assert!(*width > 0.0 && *height > 0.0);
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
        style: TextStyle { bold: false, italic: false, strikethrough: false, size: heading_size, color: [0, 0, 0], font_family: "sans-serif".to_string() },
        link_target: None,
    };
    let ast = vec![
        BlockNode::MermaidDiagram { id: "d1".to_string(), source: "flowchart TD\n A-->B".to_string(), line: 1, column: 1, file: None },
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

use sardown_ast::LinkTarget;

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

use sardown_layout::layout_impl;
use sardown_style::Stylesheet;

#[test]
fn a_larger_space_before_factor_increases_the_gap_before_a_heading() {
    let ast = vec![
        BlockNode::Paragraph { content: vec![plain_inline("End of section one.")] },
        BlockNode::Heading { level: 2, id: "two".to_string(), content: vec![sized_inline("Section Two", 22.0)] },
        BlockNode::Paragraph { content: vec![plain_inline("Start of section two.")] },
    ];
    let y_of = |pages: &[sardown_layout::PositionedPage], text: &str| {
        pages[0]
            .elements
            .iter()
            .find_map(|e| match e {
                PositionedElement::TextRun { y, text: t, .. } if t == text => Some(*y),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing text run {text:?}"))
    };

    let mut small_style = Stylesheet::default();
    small_style.heading.space_before_factor = 0.2;
    let mut fs_small = test_font_system();
    let small_output = layout_impl(&ast, &letter_geometry(), &mut fs_small, &fixtures_dir(), &DiagramTable::new(), &small_style);
    let small_gap = y_of(&small_output.pages, "Section Two") - y_of(&small_output.pages, "End of section one.");

    let mut large_style = Stylesheet::default();
    large_style.heading.space_before_factor = 1.5;
    let mut fs_large = test_font_system();
    let large_output = layout_impl(&ast, &letter_geometry(), &mut fs_large, &fixtures_dir(), &DiagramTable::new(), &large_style);
    let large_gap = y_of(&large_output.pages, "Section Two") - y_of(&large_output.pages, "End of section one.");

    assert!(large_gap > small_gap, "expected a larger space_before_factor to produce a bigger gap ({small_gap} vs {large_gap})");
}

#[test]
fn layout_still_matches_layout_impl_with_the_default_factor() {
    let ast = vec![
        BlockNode::Paragraph { content: vec![plain_inline("End of section one.")] },
        BlockNode::Heading { level: 2, id: "two".to_string(), content: vec![sized_inline("Section Two", 22.0)] },
    ];
    let mut fs_a = test_font_system();
    let via_layout = layout(&ast, &letter_geometry(), &mut fs_a, &fixtures_dir(), &DiagramTable::new());
    let mut fs_b = test_font_system();
    let via_impl = layout_impl(&ast, &letter_geometry(), &mut fs_b, &fixtures_dir(), &DiagramTable::new(), &Stylesheet::default());

    let y = |pages: &[sardown_layout::PositionedPage]| {
        pages[0]
            .elements
            .iter()
            .find_map(|e| match e {
                PositionedElement::TextRun { y, text, .. } if text == "Section Two" => Some(*y),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(y(&via_layout.pages), y(&via_impl.pages));
}

#[test]
fn blockquote_border_uses_the_configured_color_and_width() {
    let mut style = Stylesheet::default();
    style.blockquote.border_color = sardown_style::Color([9, 9, 9]);
    style.blockquote.border_width_pt = 5.0;
    let ast = vec![BlockNode::Blockquote { content: vec![BlockNode::Paragraph { content: vec![plain_inline("Quoted")] }] }];
    let mut fs = test_font_system();
    let output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);

    let (color, width) = output.pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::Path { stroke: Some(s), .. } => Some((s.color, s.width)),
            _ => None,
        })
        .expect("expected the blockquote border path");
    assert_eq!(color, [9, 9, 9]);
    assert_eq!(width, 5.0);
}

#[test]
fn blockquote_indent_uses_the_configured_value() {
    let mut narrow = Stylesheet::default();
    narrow.blockquote.indent_pt = 5.0;
    let mut wide = Stylesheet::default();
    wide.blockquote.indent_pt = 50.0;

    let ast = vec![BlockNode::Blockquote { content: vec![BlockNode::Paragraph { content: vec![plain_inline("Quoted")] }] }];
    let x_of = |pages: &[sardown_layout::PositionedPage]| {
        pages[0]
            .elements
            .iter()
            .find_map(|e| match e {
                PositionedElement::TextRun { x, text, .. } if text == "Quoted" => Some(*x),
                _ => None,
            })
            .unwrap()
    };

    let mut fs_a = test_font_system();
    let narrow_output = layout_impl(&ast, &letter_geometry(), &mut fs_a, &fixtures_dir(), &DiagramTable::new(), &narrow);
    let mut fs_b = test_font_system();
    let wide_output = layout_impl(&ast, &letter_geometry(), &mut fs_b, &fixtures_dir(), &DiagramTable::new(), &wide);

    assert!(x_of(&wide_output.pages) > x_of(&narrow_output.pages));
}

#[test]
fn thematic_break_uses_the_configured_color_and_width() {
    let mut style = Stylesheet::default();
    style.thematic_break.color = sardown_style::Color([7, 7, 7]);
    style.thematic_break.width_pt = 3.0;
    let ast = vec![BlockNode::ThematicBreak];
    let mut fs = test_font_system();
    let output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);

    let (color, width) = output.pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::Path { stroke: Some(s), .. } => Some((s.color, s.width)),
            _ => None,
        })
        .expect("expected the thematic break path");
    assert_eq!(color, [7, 7, 7]);
    assert_eq!(width, 3.0);
}

#[test]
fn list_indent_uses_the_configured_value() {
    let mut narrow = Stylesheet::default();
    narrow.list.indent_pt = 5.0;
    let mut wide = Stylesheet::default();
    wide.list.indent_pt = 50.0;

    let ast = vec![BlockNode::List { ordered: false, start: None, items: vec![vec![BlockNode::Paragraph { content: vec![plain_inline("Item")] }]] }];
    let x_of = |pages: &[sardown_layout::PositionedPage]| {
        pages[0]
            .elements
            .iter()
            .find_map(|e| match e {
                PositionedElement::TextRun { x, text, .. } if text.contains("Item") => Some(*x),
                _ => None,
            })
            .unwrap()
    };

    let mut fs_a = test_font_system();
    let narrow_output = layout_impl(&ast, &letter_geometry(), &mut fs_a, &fixtures_dir(), &DiagramTable::new(), &narrow);
    let mut fs_b = test_font_system();
    let wide_output = layout_impl(&ast, &letter_geometry(), &mut fs_b, &fixtures_dir(), &DiagramTable::new(), &wide);

    assert!(x_of(&wide_output.pages) > x_of(&narrow_output.pages));
}

#[test]
fn table_cell_padding_uses_the_configured_value() {
    let mut narrow = Stylesheet::default();
    narrow.table.cell_padding_pt = 2.0;
    let mut wide = Stylesheet::default();
    wide.table.cell_padding_pt = 60.0;

    let headers = vec![vec![plain_inline("H")]];
    let rows = vec![vec![vec![plain_inline("Cell")]]];
    let ast = vec![BlockNode::Table { headers, rows, alignments: vec![sardown_ast::ColumnAlignment::None] }];

    let x_of = |pages: &[sardown_layout::PositionedPage]| {
        pages[0]
            .elements
            .iter()
            .find_map(|e| match e {
                PositionedElement::TextRun { x, text, .. } if text == "Cell" => Some(*x),
                _ => None,
            })
            .unwrap()
    };

    let mut fs_a = test_font_system();
    let narrow_output = layout_impl(&ast, &letter_geometry(), &mut fs_a, &fixtures_dir(), &DiagramTable::new(), &narrow);
    let mut fs_b = test_font_system();
    let wide_output = layout_impl(&ast, &letter_geometry(), &mut fs_b, &fixtures_dir(), &DiagramTable::new(), &wide);

    assert!(x_of(&wide_output.pages) > x_of(&narrow_output.pages));
}

#[test]
fn table_min_row_height_uses_the_configured_value() {
    let mut short = Stylesheet::default();
    short.table.min_row_height_pt = 15.0;
    let mut tall = Stylesheet::default();
    tall.table.min_row_height_pt = 100.0;

    let headers = vec![vec![plain_inline("H")]];
    let rows = vec![vec![vec![plain_inline("Cell")]], vec![vec![plain_inline("Cell2")]]];
    let ast = vec![BlockNode::Table { headers, rows, alignments: vec![sardown_ast::ColumnAlignment::None] }];

    let y_of = |pages: &[sardown_layout::PositionedPage], text: &str| {
        pages[0]
            .elements
            .iter()
            .find_map(|e| match e {
                PositionedElement::TextRun { y, text: t, .. } if t == text => Some(*y),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing text run {text:?}"))
    };

    let mut fs_a = test_font_system();
    let short_output = layout_impl(&ast, &letter_geometry(), &mut fs_a, &fixtures_dir(), &DiagramTable::new(), &short);
    let short_gap = y_of(&short_output.pages, "Cell2") - y_of(&short_output.pages, "Cell");

    let mut fs_b = test_font_system();
    let tall_output = layout_impl(&ast, &letter_geometry(), &mut fs_b, &fixtures_dir(), &DiagramTable::new(), &tall);
    let tall_gap = y_of(&tall_output.pages, "Cell2") - y_of(&tall_output.pages, "Cell");

    assert!(tall_gap > short_gap, "expected a taller min_row_height_pt to produce a bigger row-to-row gap ({short_gap} vs {tall_gap})");
}

#[test]
fn code_block_background_uses_the_per_language_override() {
    let mut style = Stylesheet::default();
    style
        .code_block
        .languages
        .insert("rust".to_string(), sardown_style::CodeLanguageStyle { background: Some(sardown_style::Color([1, 2, 3])), ..Default::default() });
    let ast = vec![BlockNode::CodeBlock {
        language: Some("rust".to_string()),
        tokens: vec![sardown_ast::HighlightedToken { text: "fn main() {}\n".to_string(), color: [0, 0, 0] }],
    }];
    let mut fs = test_font_system();
    let output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);

    let fill = output.pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::Path { fill: Some(c), .. } => Some(*c),
            _ => None,
        })
        .expect("expected the code block background path");
    assert_eq!(fill, [1, 2, 3]);
}

#[test]
fn code_block_font_size_uses_the_per_language_override() {
    let mut style = Stylesheet::default();
    style.code_block.languages.insert("rust".to_string(), sardown_style::CodeLanguageStyle { font_size_pt: Some(20.0), ..Default::default() });
    let ast = vec![BlockNode::CodeBlock {
        language: Some("rust".to_string()),
        tokens: vec![sardown_ast::HighlightedToken { text: "fn main() {}\n".to_string(), color: [0, 0, 0] }],
    }];
    let mut fs = test_font_system();
    let output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);

    let size = output.pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { size, .. } => Some(*size),
            _ => None,
        })
        .expect("expected a code text run");
    assert_eq!(size, 20.0);
}

#[test]
fn inline_label_style_prepends_the_label_as_the_first_line_of_code_text() {
    let mut style = Stylesheet::default();
    style.code_block.label_style = sardown_style::LabelStyle::Inline;
    let ast = vec![BlockNode::CodeBlock {
        language: Some("rust".to_string()),
        tokens: vec![sardown_ast::HighlightedToken { text: "fn main() {}\n".to_string(), color: [0, 0, 0] }],
    }];
    let mut fs = test_font_system();
    let output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);

    let label_run = output.pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { text, color, .. } if text.contains("Rust") => Some(*color),
            _ => None,
        })
        .expect("expected a text run containing the auto-generated \"Rust\" label");
    assert_eq!(label_run, style.code_block.default.label_color.0);
}

#[test]
fn label_style_none_never_adds_a_label_line() {
    let style = Stylesheet::default(); // label_style defaults to None
    let ast = vec![BlockNode::CodeBlock {
        language: Some("rust".to_string()),
        tokens: vec![sardown_ast::HighlightedToken { text: "fn main() {}\n".to_string(), color: [0, 0, 0] }],
    }];
    let mut fs = test_font_system();
    let output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);

    let has_label_text = output.pages[0].elements.iter().any(|e| match e {
        PositionedElement::TextRun { text, .. } => text.contains("Rust"),
        _ => false,
    });
    assert!(!has_label_text, "expected no label text when label_style is None");
}

#[test]
fn header_bar_label_style_draws_a_background_and_label_before_the_code() {
    let mut style = Stylesheet::default();
    style.code_block.label_style = sardown_style::LabelStyle::HeaderBar;
    let ast = vec![BlockNode::CodeBlock {
        language: Some("rust".to_string()),
        tokens: vec![sardown_ast::HighlightedToken { text: "fn main() {}\n".to_string(), color: [0, 0, 0] }],
    }];
    let mut fs = test_font_system();
    let output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);

    let fills: Vec<_> = output.pages[0]
        .elements
        .iter()
        .filter_map(|e| match e {
            PositionedElement::Path { fill: Some(c), .. } => Some(*c),
            _ => None,
        })
        .collect();
    assert!(fills.contains(&style.code_block.default.label_background.0), "expected a header bar background rect, got fills: {fills:?}");

    let label_run = output.pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { text, y, color, .. } if text.contains("Rust") => Some((*y, *color)),
            _ => None,
        })
        .expect("expected a text run containing the auto-generated \"Rust\" label");
    assert_eq!(label_run.1, style.code_block.default.label_color.0);

    let code_text_y = output.pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { text, y, .. } if text.contains("fn") => Some(*y),
            _ => None,
        })
        .expect("expected the code's own text run");
    assert!(label_run.0 < code_text_y, "expected the header bar label above the code's own text");
}

#[test]
fn corner_label_style_draws_a_badge_overlapping_the_code_backgrounds_top_edge() {
    let mut style = Stylesheet::default();
    style.code_block.label_style = sardown_style::LabelStyle::Corner;
    let ast = vec![BlockNode::CodeBlock {
        language: Some("rust".to_string()),
        tokens: vec![sardown_ast::HighlightedToken { text: "fn main() {}\n".to_string(), color: [0, 0, 0] }],
    }];
    let mut fs = test_font_system();
    let output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);

    let fills: Vec<_> = output.pages[0]
        .elements
        .iter()
        .filter_map(|e| match e {
            PositionedElement::Path { fill: Some(c), .. } => Some(*c),
            _ => None,
        })
        .collect();
    assert!(fills.contains(&style.code_block.default.label_background.0), "expected a corner badge background rect, got fills: {fills:?}");

    let label_run = output.pages[0].elements.iter().any(|e| match e {
        PositionedElement::TextRun { text, color, .. } => text.contains("Rust") && *color == style.code_block.default.label_color.0,
        _ => false,
    });
    assert!(label_run, "expected a \"Rust\" text run in the label color");
}

#[test]
fn corner_label_gives_the_start_page_extra_top_padding() {
    let plain = Stylesheet::default();
    let mut corner = Stylesheet::default();
    corner.code_block.label_style = sardown_style::LabelStyle::Corner;

    let ast = vec![BlockNode::CodeBlock {
        language: Some("rust".to_string()),
        tokens: vec![sardown_ast::HighlightedToken { text: "fn main() {}\n".to_string(), color: [0, 0, 0] }],
    }];

    let background_top_y = |output: &sardown_layout::LayoutOutput| {
        output.pages[0]
            .elements
            .iter()
            .find_map(|e| match e {
                PositionedElement::Path { points, fill: Some(_), .. } => match points.first() {
                    Some(sardown_layout::PathCommand::MoveTo(_, y)) => Some(*y),
                    _ => None,
                },
                _ => None,
            })
            .expect("expected a filled background path")
    };

    let mut fs_a = test_font_system();
    let plain_output = layout_impl(&ast, &letter_geometry(), &mut fs_a, &fixtures_dir(), &DiagramTable::new(), &plain);
    let mut fs_b = test_font_system();
    let corner_output = layout_impl(&ast, &letter_geometry(), &mut fs_b, &fixtures_dir(), &DiagramTable::new(), &corner);

    assert!(
        background_top_y(&corner_output) < background_top_y(&plain_output),
        "expected the corner style's code background to start higher up (larger top pad) than the plain style's"
    );
}

#[test]
fn headings_and_code_blocks_stay_left_aligned_even_under_a_justified_stylesheet() {
    let mut left_style = sardown_style::Stylesheet::default();
    left_style.typography.alignment = sardown_style::TextAlignment::Left;
    let mut justified_style = sardown_style::Stylesheet::default();
    justified_style.typography.alignment = sardown_style::TextAlignment::Justify;

    let ast = vec![
        BlockNode::Heading {
            level: 2,
            id: "h".to_string(),
            content: vec![sized_inline("A somewhat longer heading that wraps across more than one line here", 22.0)],
        },
        BlockNode::CodeBlock {
            language: None,
            tokens: vec![HighlightedToken { text: "some code text that is long enough to wrap across more than one line".to_string(), color: [0, 0, 0] }],
        },
    ];

    let mut fs_left = test_font_system();
    let left_output = layout_impl(&ast, &letter_geometry(), &mut fs_left, &fixtures_dir(), &DiagramTable::new(), &left_style);
    let mut fs_justified = test_font_system();
    let justified_output = layout_impl(&ast, &letter_geometry(), &mut fs_justified, &fixtures_dir(), &DiagramTable::new(), &justified_style);

    let text_run_positions = |pages: &[sardown_layout::PositionedPage]| -> Vec<(f32, f32)> {
        pages
            .iter()
            .flat_map(|p| &p.elements)
            .filter_map(|e| match e {
                PositionedElement::TextRun { x, y, .. } => Some((*x, *y)),
                _ => None,
            })
            .collect()
    };

    assert_eq!(
        text_run_positions(&left_output.pages),
        text_run_positions(&justified_output.pages),
        "expected heading and code block glyph positions to be identical regardless of typography.alignment"
    );
}

#[test]
fn a_heading_honors_center_and_right_alignment() {
    let ast = vec![BlockNode::Heading { level: 1, id: "h".to_string(), content: vec![sized_inline("Hi", 28.0)] }];

    let mut left_style = Stylesheet::default();
    left_style.typography.alignment = sardown_style::TextAlignment::Left;
    let mut fs = test_font_system();
    let left_x = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &left_style).pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { x, .. } => Some(*x),
            _ => None,
        })
        .unwrap();

    let mut center_style = Stylesheet::default();
    center_style.typography.alignment = sardown_style::TextAlignment::Center;
    let mut fs = test_font_system();
    let center_x = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &center_style).pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { x, .. } => Some(*x),
            _ => None,
        })
        .unwrap();

    let mut right_style = Stylesheet::default();
    right_style.typography.alignment = sardown_style::TextAlignment::Right;
    let mut fs = test_font_system();
    let right_x = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &right_style).pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { x, .. } => Some(*x),
            _ => None,
        })
        .unwrap();

    assert!(center_x > left_x, "expected a centered heading to start further right than a left-aligned one");
    assert!(right_x > center_x, "expected a right-aligned heading to start further right than a centered one");
}

#[test]
fn strikethrough_text_draws_a_horizontal_line_through_it() {
    let mut plain = plain_inline("plain");
    let mut struck = plain_inline("struck");
    struck.style.strikethrough = true;
    plain.style.strikethrough = false;
    let ast = vec![BlockNode::Paragraph { content: vec![plain, struck] }];

    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;

    // `TextRun::text` holds the *whole shaped line's* text for every run on that line, not just
    // that run's own substring (confirmed empirically -- both spans' TextRuns contain
    // "plainstruck"), so runs can't be told apart by text content. "plain" and "struck" are
    // adjacent with no space between them, so they land as two contiguous TextRuns in source
    // order: the first (smaller x) is "plain", the second (larger x, picking up exactly where
    // "plain" ends) is "struck".
    let mut text_runs: Vec<(f32, f32, f32)> = pages[0]
        .elements
        .iter()
        .filter_map(|e| match e {
            PositionedElement::TextRun { x, y, glyphs, .. } => Some((*x, *y, glyphs.iter().map(|g| g.x_advance).sum::<f32>())),
            _ => None,
        })
        .collect();
    text_runs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    assert_eq!(text_runs.len(), 2, "expected exactly one TextRun per span, got {text_runs:?}");
    let plain_run_x = (text_runs[0].0, text_runs[0].1);
    let struck_run = text_runs[1];

    let (struck_x, struck_y, struck_width) = struck_run;
    let strike_line = pages[0].elements.iter().find_map(|e| match e {
        PositionedElement::Path { points, stroke: Some(_), .. } => match points.as_slice() {
            [sardown_layout::PathCommand::MoveTo(x0, y0), sardown_layout::PathCommand::LineTo(x1, y1)] if y0 == y1 => Some((*x0, *y0, *x1)),
            _ => None,
        },
        _ => None,
    });

    let (line_x0, line_y, line_x1) = strike_line.expect("expected a horizontal strikethrough line path");
    assert!(
        line_x0 >= struck_x - 0.5 && line_x1 <= struck_x + struck_width + 0.5,
        "expected the line to span the struck-through run's own width, got {line_x0}..{line_x1} vs run {struck_x}..{}",
        struck_x + struck_width
    );
    assert!(line_y < struck_y, "expected the strikethrough line to sit above the text baseline");
    assert_ne!((line_x0, line_y), (plain_run_x.0, plain_run_x.1), "the line shouldn't be positioned at the plain run's own coordinates");
}

fn any_line_ends_in_a_hyphen(output: &sardown_layout::LayoutOutput) -> bool {
    // `TextRun::text` holds the whole *buffer line*'s text (here, an entire paragraph, since a
    // hyphenated break is a plain "-" with no forced line-break character -- see the design note
    // in hyphenate.rs), not just this run's own visually-wrapped substring. Extract this run's
    // real substring from its own glyphs' cluster ranges before checking for a trailing hyphen.
    output.pages.iter().flat_map(|p| &p.elements).any(|e| match e {
        PositionedElement::TextRun { text, glyphs, .. } => {
            let Some(min_start) = glyphs.iter().map(|g| g.cluster.start).min() else { return false };
            let Some(max_end) = glyphs.iter().map(|g| g.cluster.end).max() else { return false };
            text[min_start..max_end].ends_with('-')
        }
        _ => false,
    })
}

#[test]
fn hyphenation_enabled_in_the_stylesheet_splits_a_long_word_across_lines() {
    let mut style = Stylesheet::default();
    style.typography.hyphenation = true;
    style.typography.language = "en-us".to_string();
    let ast = parse("An extraordinarily long hyphenation demonstration paragraph that must wrap.\n");
    let mut fs = test_font_system();
    let narrow_geometry = PageGeometry { page_width_mm: 40.0, page_height_mm: 279.4, margin_mm: 5.0, ..Default::default() };
    let output = layout_impl(&ast, &narrow_geometry, &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);

    assert!(any_line_ends_in_a_hyphen(&output), "expected at least one line to end in a hyphenated word break");
}

#[test]
fn hyphenation_disabled_by_default_produces_no_hyphenated_breaks() {
    let style = Stylesheet::default(); // hyphenation: false
    let ast = parse("An extraordinarily long hyphenation demonstration paragraph that must wrap.\n");
    let mut fs = test_font_system();
    let narrow_geometry = PageGeometry { page_width_mm: 40.0, page_height_mm: 279.4, margin_mm: 5.0, ..Default::default() };
    let output = layout_impl(&ast, &narrow_geometry, &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);

    assert!(!any_line_ends_in_a_hyphen(&output), "expected no hyphenation to occur when typography.hyphenation is false");
}

#[test]
fn hyphenation_does_not_apply_to_headings() {
    let mut style = Stylesheet::default();
    style.typography.hyphenation = true;
    let ast = parse("# An Extraordinarily Long Hyphenation Demonstration Heading\n");
    let mut fs = test_font_system();
    let narrow_geometry = PageGeometry { page_width_mm: 40.0, page_height_mm: 279.4, margin_mm: 5.0, ..Default::default() };
    let output = layout_impl(&ast, &narrow_geometry, &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);

    assert!(!any_line_ends_in_a_hyphen(&output), "expected headings to never be hyphenated, even with typography.hyphenation = true");
}

#[test]
fn a_raster_image_taller_than_the_page_is_capped_to_the_available_height() {
    // Regression test: unlike embedded SVGs and Mermaid diagrams (both placed via
    // fit_vector_graphic, which caps by width *and* height), a raster image was only ever capped
    // by width -- a narrow, very tall image (tests/fixtures/tall-image.png is 100x2000px, aspect
    // 20:1) scaled to fit a normal content width still ends up far taller than any page, with no
    // second pass to cap it back down. tall-image.png alone on a US Letter page (content width
    // ~165mm) would naively scale to roughly 467pt wide x 9340pt tall -- vastly taller than the
    // ~640pt of vertical space even a full fresh page provides.
    let ast =
        vec![BlockNode::Image { alt: "tall".to_string(), title: None, source: sardown_ast::ImageSource::Embedded(std::path::PathBuf::from("tall-image.png")) }];
    let mut fs = test_font_system();
    let geometry = letter_geometry();
    let output = layout(&ast, &geometry, &mut fs, &fixtures_dir(), &DiagramTable::new());

    let margin_pt = geometry.margin_mm * 2.834_645_7;
    let page_height_pt = geometry.page_height_mm * 2.834_645_7;
    let max_height_pt = page_height_pt - margin_pt - margin_pt; // top margin to bottom margin

    match &output.pages[0].elements[0] {
        PositionedElement::RasterImage { width, height, .. } => {
            assert!(*height <= max_height_pt + 0.5, "expected the image's height to be capped to the page's available height ({max_height_pt}), got {height}");
            let aspect = 2000.0 / 100.0;
            assert!(
                (*width - *height / aspect).abs() < 0.5,
                "expected the aspect ratio to still be preserved after height-capping: width={width}, height={height}"
            );
        }
        other => panic!("expected RasterImage, got {other:?}"),
    }
}

fn text_run_x_positions(page: &sardown_layout::PositionedPage) -> Vec<f32> {
    page.elements
        .iter()
        .filter_map(|e| match e {
            PositionedElement::TextRun { x, .. } => Some(*x),
            _ => None,
        })
        .collect()
}

#[test]
fn two_columns_render_at_non_overlapping_x_offsets() {
    let ast = vec![BlockNode::Columns(vec![
        vec![BlockNode::Paragraph { content: vec![plain_inline("Left")] }],
        vec![BlockNode::Paragraph { content: vec![plain_inline("Right")] }],
    ])];
    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());
    let xs = text_run_x_positions(&output.pages[0]);
    assert_eq!(xs.len(), 2, "expected one text run per column");
    assert!(xs[1] > xs[0], "expected the right column's text to start further right than the left column's, got {xs:?}");

    let margin_pt = letter_geometry().margin_mm * 2.834_645_7;
    let content_width_pt = letter_geometry().page_width_mm * 2.834_645_7 - 2.0 * margin_pt;
    let expected_column_width_pt = (content_width_pt - 24.0) / 2.0; // default columns.gap_pt = 24.0
    assert!(
        (xs[1] - xs[0] - (expected_column_width_pt + 24.0)).abs() < 1.0,
        "expected the right column to start one column-width-plus-gap after the left column, got left={} right={}",
        xs[0],
        xs[1]
    );
}

#[test]
fn columns_block_height_is_the_tallest_column_not_the_sum() {
    let ast = vec![
        BlockNode::Columns(vec![
            vec![BlockNode::Paragraph { content: vec![plain_inline("One short line.")] }],
            vec![
                BlockNode::Paragraph { content: vec![plain_inline("Line one.")] },
                BlockNode::Paragraph { content: vec![plain_inline("Line two.")] },
                BlockNode::Paragraph { content: vec![plain_inline("Line three.")] },
            ],
        ]),
        BlockNode::Paragraph { content: vec![plain_inline("After the columns.")] },
    ];
    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());
    // Every paragraph fits comfortably on one Letter page -- if height were wrongly summed
    // instead of maxed, this would still likely fit on one page too, so the real assertion is
    // on the "After the columns." paragraph's own y position, checked against the taller
    // column's own last line, not against a summed height further down below.
    let after_y = output.pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::TextRun { y, text, .. } if text.contains("After the columns.") => Some(*y),
            _ => None,
        })
        .expect("expected to find the trailing paragraph's own text run");
    let column_ys: Vec<f32> = output.pages[0]
        .elements
        .iter()
        .filter_map(|e| match e {
            PositionedElement::TextRun { y, text, .. } if !text.contains("After the columns.") => Some(*y),
            _ => None,
        })
        .collect();
    let max_column_y = column_ys.iter().cloned().fold(f32::MIN, f32::max);
    assert!(after_y > max_column_y, "expected the trailing paragraph to sit below every column's own text");
}

#[test]
fn a_column_can_contain_a_list_and_another_a_code_block_full_block_type_reuse() {
    let ast = vec![BlockNode::Columns(vec![
        vec![BlockNode::List { ordered: false, start: None, items: vec![vec![BlockNode::Paragraph { content: vec![plain_inline("Bullet")] }]] }],
        vec![BlockNode::CodeBlock { language: None, tokens: vec![HighlightedToken { text: "code".to_string(), color: [0, 0, 0] }] }],
    ])];
    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());
    let has_bullet_marker = output.pages[0].elements.iter().any(|e| matches!(e, PositionedElement::TextRun { text, .. } if text.contains('\u{2022}')));
    let has_code_background = output.pages[0].elements.iter().any(|e| matches!(e, PositionedElement::Path { fill: Some(_), .. }));
    assert!(has_bullet_marker, "expected the list's own bullet marker to render inside its column");
    assert!(has_code_background, "expected the code block's own background to render inside its column");
}

#[test]
fn an_anchor_inside_a_column_resolves_to_the_columns_own_page() {
    let ast =
        vec![BlockNode::Columns(vec![vec![BlockNode::Heading { level: 2, id: "in-column".to_string(), content: vec![sized_inline("In Column", 22.0)] }]])];
    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());
    let anchor = output.anchors.get("in-column").expect("expected the heading anchor to be recorded");
    assert_eq!(anchor.page, 0);
    assert!(anchor.x > 0.0 && anchor.y > 0.0);
}

#[test]
fn a_page_break_inside_a_column_keeps_only_the_first_internal_page_without_panicking() {
    // A `::columns` column is rendered against an isolated Cursor on the documented assumption
    // that it always produces exactly one internal page (see the Columns arm's own doc comment).
    // A BlockNode::PageBreak inside a column violates that assumption (break_page is unconditional,
    // independent of remaining height) -- not reachable via any current producer of PageBreak
    // (only sardown-book's chapter-combination inserts it, and sardown-book never calls
    // group_columns), but BlockNode::Columns/group_columns are general-purpose AST/layout API,
    // not gated to slides-only use. This locks in the documented, non-silent fallback: the first
    // internal page's content is kept, content after the break is dropped, and this must never
    // panic.
    let ast = vec![BlockNode::Columns(vec![vec![
        BlockNode::Paragraph { content: vec![plain_inline("Before the break.")] },
        BlockNode::PageBreak,
        BlockNode::Paragraph { content: vec![plain_inline("After the break.")] },
    ]])];
    let mut fs = test_font_system();
    let output = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new());
    let text_of = |page: &sardown_layout::PositionedPage| {
        page.elements
            .iter()
            .filter_map(|e| match e {
                PositionedElement::TextRun { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let full_text = text_of(&output.pages[0]);
    assert!(full_text.contains("Before the break."), "expected the first internal page's content to survive: {full_text}");
    assert!(!full_text.contains("After the break."), "content after an internal page break inside a column is documented as dropped, not stacked: {full_text}");
}

#[test]
fn a_heading_with_no_underline_configured_draws_no_extra_path() {
    let ast = parse("# A Heading\n");
    let mut fs = test_font_system();
    let pages = layout(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new()).pages;
    let has_path = pages[0].elements.iter().any(|e| matches!(e, PositionedElement::Path { .. }));
    assert!(!has_path, "default heading style has underline_width_pt = 0.0 -- expected no Path element");
}

#[test]
fn a_heading_with_an_underline_configured_draws_a_stroked_path_using_its_color_and_width() {
    let mut style = Stylesheet::default();
    style.heading.underline_width_pt = 2.0;
    style.heading.underline_color = sardown_style::Color([9, 9, 9]);
    let ast = parse("# A Heading\n");
    let mut fs = test_font_system();
    let output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);

    let (color, width) = output.pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::Path { stroke: Some(s), .. } => Some((s.color, s.width)),
            _ => None,
        })
        .expect("expected an underline path under the heading");
    assert_eq!(color, [9, 9, 9]);
    assert_eq!(width, 2.0);
}

#[test]
fn a_headings_underline_hugs_the_headings_own_text_width_not_the_full_content_width() {
    use sardown_layout::PathCommand;
    let mut style = Stylesheet::default();
    style.heading.underline_width_pt = 2.0;
    // A very short heading on a wide (letter-size) page: the underline must stop well short of
    // the full content width, matching how a block-level heading sized to its own content (not
    // stretched to fill its container) actually looks.
    let ast = parse("# Hi\n");
    let mut fs = test_font_system();
    let output = layout_impl(&ast, &letter_geometry(), &mut fs, &fixtures_dir(), &DiagramTable::new(), &style);

    let margin_pt = letter_geometry().margin_mm * 2.834_645_7;
    let content_width_pt = letter_geometry().page_width_mm * 2.834_645_7 - 2.0 * margin_pt;

    let underline_length_pt = output.pages[0]
        .elements
        .iter()
        .find_map(|e| match e {
            PositionedElement::Path { points, stroke: Some(_), .. } => match points.as_slice() {
                [PathCommand::MoveTo(x0, _), PathCommand::LineTo(x1, _)] => Some(x1 - x0),
                _ => None,
            },
            _ => None,
        })
        .expect("expected an underline path under the heading");
    assert!(
        underline_length_pt < content_width_pt * 0.5,
        "expected the underline to hug \"Hi\"'s own short width, not the full content width \
         ({content_width_pt}pt); got {underline_length_pt}pt"
    );
}
