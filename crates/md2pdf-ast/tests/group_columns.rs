use md2pdf_ast::{group_columns, BlockNode, InlineNode, TextStyle};

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

fn heading(text: &str) -> BlockNode {
    BlockNode::Heading { level: 1, id: text.to_lowercase(), content: vec![plain(text)] }
}

#[test]
fn two_columns_split_correctly() {
    let blocks = vec![
        paragraph("::columns"),
        paragraph("::column"),
        paragraph("Left A"),
        paragraph("Left B"),
        paragraph("::column"),
        paragraph("Right A"),
        paragraph("::end"),
    ];
    let grouped = group_columns(blocks);
    assert_eq!(grouped.len(), 1);
    let BlockNode::Columns(columns) = &grouped[0] else { panic!("expected Columns, got {:?}", grouped[0]) };
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0], vec![paragraph("Left A"), paragraph("Left B")]);
    assert_eq!(columns[1], vec![paragraph("Right A")]);
}

#[test]
fn sentinel_paragraphs_are_removed_from_the_final_blocks() {
    let blocks = vec![paragraph("::columns"), paragraph("::column"), paragraph("Content"), paragraph("::end")];
    let grouped = group_columns(blocks);
    let BlockNode::Columns(columns) = &grouped[0] else { panic!("expected Columns") };
    assert_eq!(columns[0], vec![paragraph("Content")]);
}

#[test]
fn no_end_before_the_block_list_ends_still_groups_everything_seen() {
    let blocks = vec![paragraph("::columns"), paragraph("::column"), paragraph("Left"), paragraph("::column"), paragraph("Right")];
    let grouped = group_columns(blocks);
    let BlockNode::Columns(columns) = &grouped[0] else { panic!("expected Columns") };
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0], vec![paragraph("Left")]);
    assert_eq!(columns[1], vec![paragraph("Right")]);
}

#[test]
fn a_single_column_marker_produces_a_one_column_result() {
    let blocks = vec![paragraph("::columns"), paragraph("::column"), paragraph("Only"), paragraph("::end")];
    let grouped = group_columns(blocks);
    let BlockNode::Columns(columns) = &grouped[0] else { panic!("expected Columns") };
    assert_eq!(columns.len(), 1);
    assert_eq!(columns[0], vec![paragraph("Only")]);
}

#[test]
fn content_with_no_column_marker_at_all_becomes_one_implicit_column_never_dropped() {
    let blocks = vec![paragraph("::columns"), paragraph("Stray content"), paragraph("::end")];
    let grouped = group_columns(blocks);
    let BlockNode::Columns(columns) = &grouped[0] else { panic!("expected Columns") };
    assert_eq!(columns.len(), 1, "no ::column markers at all -- everything becomes one implicit column, never dropped");
    assert_eq!(columns[0], vec![paragraph("Stray content")]);
}

#[test]
fn non_sentinel_paragraphs_that_merely_start_with_the_sentinel_text_are_left_untouched() {
    let blocks = vec![paragraph("::columns are neat"), paragraph("Body")];
    let grouped = group_columns(blocks.clone());
    assert_eq!(grouped, blocks, "exact match only -- text that merely starts with the sentinel is ordinary content");
}

#[test]
fn a_bare_column_marker_outside_any_columns_block_is_left_as_an_ordinary_paragraph() {
    let blocks = vec![paragraph("::column"), paragraph("Body")];
    let grouped = group_columns(blocks.clone());
    assert_eq!(grouped, blocks);
}

#[test]
fn a_nested_columns_sentinel_inside_a_column_is_treated_as_literal_text_not_a_new_block() {
    let blocks = vec![
        paragraph("::columns"),
        paragraph("::column"),
        paragraph("::columns"), // typo'd by an author -- not specially recognized while already inside a block
        paragraph("::end"),
    ];
    let grouped = group_columns(blocks);
    assert_eq!(grouped.len(), 1);
    let BlockNode::Columns(columns) = &grouped[0] else { panic!("expected Columns") };
    assert_eq!(columns.len(), 1);
    assert_eq!(columns[0], vec![paragraph("::columns")], "the nested sentinel-shaped paragraph is kept as literal content");
}

#[test]
fn headings_and_other_block_types_pass_through_columns_grouping_unaffected() {
    let blocks = vec![heading("Title"), paragraph("::columns"), paragraph("::column"), paragraph("Body"), paragraph("::end")];
    let grouped = group_columns(blocks);
    assert_eq!(grouped[0], heading("Title"));
    assert!(matches!(grouped[1], BlockNode::Columns(_)));
}
