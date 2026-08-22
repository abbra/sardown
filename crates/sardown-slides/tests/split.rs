use sardown_ast::{BlockNode, InlineNode, TextStyle};
use sardown_slides::split_into_slides;

fn plain(text: &str) -> InlineNode {
    InlineNode {
        text: text.to_string(),
        style: TextStyle { bold: false, italic: false, strikethrough: false, size: 12.0, color: [0, 0, 0], font_family: "sans-serif".into() },
        link_target: None,
    }
}

fn heading(text: &str) -> BlockNode {
    BlockNode::Heading { level: 1, id: text.to_lowercase(), content: vec![plain(text)] }
}

fn paragraph(text: &str) -> BlockNode {
    BlockNode::Paragraph { content: vec![plain(text)] }
}

#[test]
fn a_deck_with_no_thematic_breaks_is_exactly_one_slide() {
    let ast = vec![heading("Title"), paragraph("Body")];
    let slides = split_into_slides(ast);
    assert_eq!(slides.len(), 1);
    assert_eq!(slides[0].blocks.len(), 2);
}

#[test]
fn n_thematic_breaks_produce_n_plus_one_slides() {
    let ast = vec![heading("One"), BlockNode::ThematicBreak, heading("Two"), BlockNode::ThematicBreak, heading("Three")];
    let slides = split_into_slides(ast);
    assert_eq!(slides.len(), 3);
    assert_eq!(slides[0].blocks, vec![heading("One")]);
    assert_eq!(slides[1].blocks, vec![heading("Two")]);
    assert_eq!(slides[2].blocks, vec![heading("Three")]);
}

#[test]
fn a_slide_with_no_layout_directive_has_no_layout_name() {
    let slides = split_into_slides(vec![heading("Plain")]);
    assert_eq!(slides[0].layout_name, None);
}

#[test]
fn a_leading_layout_directive_paragraph_is_extracted_and_removed() {
    let ast = vec![paragraph("@layout: title"), heading("My Title")];
    let slides = split_into_slides(ast);
    assert_eq!(slides[0].layout_name, Some("title".to_string()));
    assert_eq!(slides[0].blocks, vec![heading("My Title")]);
}

#[test]
fn a_layout_directive_only_matches_as_the_slides_first_block() {
    let ast = vec![heading("My Title"), paragraph("@layout: title")];
    let slides = split_into_slides(ast);
    assert_eq!(slides[0].layout_name, None);
    assert_eq!(slides[0].blocks.len(), 2, "the directive-shaped paragraph is left in place, not stripped");
}

#[test]
fn the_directive_matches_text_concatenated_across_multiple_inline_runs() {
    let mut bold_style = plain("").style;
    bold_style.bold = true;
    let ast = vec![
        BlockNode::Paragraph { content: vec![plain("@layout: "), InlineNode { text: "title".to_string(), style: bold_style, link_target: None }] },
        heading("My Title"),
    ];
    let slides = split_into_slides(ast);
    assert_eq!(slides[0].layout_name, Some("title".to_string()));
    assert_eq!(slides[0].blocks, vec![heading("My Title")]);
}

#[test]
fn each_slides_own_layout_directive_is_resolved_independently() {
    let ast = vec![paragraph("@layout: title"), heading("First"), BlockNode::ThematicBreak, heading("Second")];
    let slides = split_into_slides(ast);
    assert_eq!(slides[0].layout_name, Some("title".to_string()));
    assert_eq!(slides[1].layout_name, None);
}
