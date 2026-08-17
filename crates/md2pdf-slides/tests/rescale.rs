use md2pdf_ast::{BlockNode, InlineNode, TextStyle};
use md2pdf_slides::rescale_slide_content;
use md2pdf_style::{Color, SlideLayoutStyle, Stylesheet};

fn plain(text: &str, size: f32) -> InlineNode {
    InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size, color: [0, 0, 0], font_family: "sans-serif".to_string() },
        link_target: None,
    }
}

fn bold(text: &str, size: f32) -> InlineNode {
    InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: true, italic: false, strikethrough: false, size, color: [0, 0, 0], font_family: "sans-serif".to_string() },
        link_target: None,
    }
}

fn sizes_in(blocks: &[BlockNode]) -> Vec<f32> {
    fn collect(blocks: &[BlockNode], out: &mut Vec<f32>) {
        for block in blocks {
            match block {
                BlockNode::Heading { content, .. } | BlockNode::Paragraph { content } => {
                    out.extend(content.iter().map(|n| n.style.size));
                }
                BlockNode::Blockquote { content } => collect(content, out),
                BlockNode::List { items, .. } => {
                    for item in items {
                        collect(item, out);
                    }
                }
                BlockNode::Table { headers, rows, .. } => {
                    for cell in headers {
                        out.extend(cell.iter().map(|n| n.style.size));
                    }
                    for row in rows {
                        for cell in row {
                            out.extend(cell.iter().map(|n| n.style.size));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    collect(blocks, &mut out);
    out
}

#[test]
fn a_paragraph_is_resized_to_the_documents_body_size_times_scale() {
    let mut blocks = vec![BlockNode::Paragraph { content: vec![plain("Body.", 12.0)] }];
    let base = Stylesheet::default();
    rescale_slide_content(&mut blocks, &base, &SlideLayoutStyle::default(), 0.5);
    assert_eq!(sizes_in(&blocks), vec![6.0]);
}

#[test]
fn a_heading_is_resized_to_its_own_levels_size_times_scale_independent_of_body_size_pt() {
    let mut blocks = vec![BlockNode::Heading { level: 1, id: "h".to_string(), content: vec![plain("Title", 28.0)] }];
    let base = Stylesheet::default();
    rescale_slide_content(&mut blocks, &base, &SlideLayoutStyle::default(), 0.5);
    assert_eq!(sizes_in(&blocks), vec![14.0], "H1 default is 28pt; scaled by 0.5 -> 14pt");
}

#[test]
fn a_layouts_body_size_pt_override_only_affects_paragraphs_not_headings() {
    let mut blocks = vec![
        BlockNode::Heading { level: 1, id: "h".to_string(), content: vec![plain("Title", 28.0)] },
        BlockNode::Paragraph { content: vec![plain("Body.", 12.0)] },
    ];
    let base = Stylesheet::default();
    let mut layout = SlideLayoutStyle::default();
    layout.body_size_pt = Some(20.0);
    rescale_slide_content(&mut blocks, &base, &layout, 1.0);
    assert_eq!(sizes_in(&blocks), vec![28.0, 20.0], "heading keeps its own 28pt size; only the paragraph adopts the override");
}

#[test]
fn a_table_cells_text_is_resized_to_the_documents_table_text_size_times_scale() {
    let blocks_before = vec![BlockNode::Table {
        headers: vec![vec![plain("Col A", 10.5)]],
        rows: vec![vec![vec![plain("Cell", 10.5)]]],
        alignments: vec![md2pdf_ast::ColumnAlignment::None],
    }];
    let mut blocks = blocks_before;
    let base = Stylesheet::default();
    rescale_slide_content(&mut blocks, &base, &SlideLayoutStyle::default(), 0.5);
    assert_eq!(sizes_in(&blocks), vec![5.25, 5.25]);
}

#[test]
fn nested_content_inside_a_list_item_is_rescaled_too() {
    let mut blocks = vec![BlockNode::List { ordered: false, start: None, items: vec![vec![BlockNode::Paragraph { content: vec![plain("Item.", 12.0)] }]] }];
    let base = Stylesheet::default();
    rescale_slide_content(&mut blocks, &base, &SlideLayoutStyle::default(), 0.5);
    assert_eq!(sizes_in(&blocks), vec![6.0]);
}

#[test]
fn a_text_color_override_applies_to_both_paragraph_and_heading_text() {
    let mut blocks = vec![
        BlockNode::Heading { level: 1, id: "h".to_string(), content: vec![plain("Title", 28.0)] },
        BlockNode::Paragraph { content: vec![plain("Body.", 12.0)] },
    ];
    let base = Stylesheet::default();
    let mut layout = SlideLayoutStyle::default();
    layout.text_color = Some(Color([255, 255, 255]));
    rescale_slide_content(&mut blocks, &base, &layout, 1.0);
    let BlockNode::Heading { content: heading_content, .. } = &blocks[0] else { unreachable!() };
    let BlockNode::Paragraph { content: paragraph_content } = &blocks[1] else { unreachable!() };
    assert_eq!(heading_content[0].style.color, [255, 255, 255]);
    assert_eq!(paragraph_content[0].style.color, [255, 255, 255]);
}

#[test]
fn secondary_text_color_applies_only_to_non_bold_paragraph_runs() {
    let mut blocks = vec![BlockNode::Paragraph { content: vec![bold("Alexander Bokovoy", 12.0), plain(" · plain tail", 12.0)] }];
    let base = Stylesheet::default();
    let mut layout = SlideLayoutStyle::default();
    layout.text_color = Some(Color([255, 255, 255]));
    layout.secondary_text_color = Some(Color([155, 138, 180]));
    rescale_slide_content(&mut blocks, &base, &layout, 1.0);
    let BlockNode::Paragraph { content } = &blocks[0] else { unreachable!() };
    assert_eq!(content[0].style.color, [255, 255, 255], "bold run keeps the primary text_color");
    assert_eq!(content[1].style.color, [155, 138, 180], "non-bold run gets the muted secondary_text_color");
}

#[test]
fn secondary_text_color_falls_back_to_text_color_when_unset() {
    let mut blocks = vec![BlockNode::Paragraph { content: vec![plain("Body.", 12.0)] }];
    let base = Stylesheet::default();
    let mut layout = SlideLayoutStyle::default();
    layout.text_color = Some(Color([255, 255, 255]));
    // secondary_text_color deliberately left unset
    rescale_slide_content(&mut blocks, &base, &layout, 1.0);
    let BlockNode::Paragraph { content } = &blocks[0] else { unreachable!() };
    assert_eq!(content[0].style.color, [255, 255, 255], "expected the plain run to fall back to text_color");
}

#[test]
fn secondary_text_color_never_applies_to_headings_even_when_bold() {
    let mut blocks = vec![BlockNode::Heading { level: 1, id: "h".to_string(), content: vec![plain("Title", 28.0)] }];
    let base = Stylesheet::default();
    let mut layout = SlideLayoutStyle::default();
    layout.text_color = Some(Color([255, 255, 255]));
    layout.secondary_text_color = Some(Color([155, 138, 180]));
    rescale_slide_content(&mut blocks, &base, &layout, 1.0);
    let BlockNode::Heading { content, .. } = &blocks[0] else { unreachable!() };
    assert_eq!(content[0].style.color, [255, 255, 255], "headings always use text_color, never secondary_text_color");
}

#[test]
fn no_text_color_override_leaves_the_original_color_untouched() {
    let mut blocks = vec![BlockNode::Paragraph { content: vec![plain("Body.", 12.0)] }];
    let base = Stylesheet::default();
    rescale_slide_content(&mut blocks, &base, &SlideLayoutStyle::default(), 1.0);
    let BlockNode::Paragraph { content } = &blocks[0] else { unreachable!() };
    assert_eq!(content[0].style.color, [0, 0, 0]);
}

#[test]
fn rescaling_recurses_into_each_column_of_a_columns_block() {
    let mut blocks = vec![BlockNode::Columns(vec![
        vec![BlockNode::Paragraph { content: vec![plain("Left.", 12.0)] }],
        vec![BlockNode::Heading { level: 1, id: "h".to_string(), content: vec![plain("Right.", 28.0)] }],
    ])];
    let base = Stylesheet::default();
    rescale_slide_content(&mut blocks, &base, &SlideLayoutStyle::default(), 0.5);
    let BlockNode::Columns(columns) = &blocks[0] else { unreachable!() };
    let BlockNode::Paragraph { content: left } = &columns[0][0] else { unreachable!() };
    let BlockNode::Heading { content: right, .. } = &columns[1][0] else { unreachable!() };
    assert_eq!(left[0].style.size, 6.0, "paragraph inside a column still resizes to body_size_pt * scale");
    assert_eq!(right[0].style.size, 14.0, "heading inside a column still resizes to its own level's size * scale");
}
