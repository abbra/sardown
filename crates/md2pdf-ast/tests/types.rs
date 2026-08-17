use md2pdf_ast::*;

#[test]
fn block_node_variants_are_constructible() {
    let heading = BlockNode::Heading {
        level: 1,
        id: "intro".to_string(),
        content: vec![InlineNode {
            text: "Intro".to_string(),
            style: TextStyle { bold: false, italic: false, strikethrough: false, size: 24.0, color: [0, 0, 0], font_family: "sans-serif".to_string() },
            link_target: None,
        }],
    };
    assert_eq!(
        heading,
        BlockNode::Heading {
            level: 1,
            id: "intro".to_string(),
            content: vec![InlineNode {
                text: "Intro".to_string(),
                style: TextStyle { bold: false, italic: false, strikethrough: false, size: 24.0, color: [0, 0, 0], font_family: "sans-serif".to_string() },
                link_target: None,
            }],
        }
    );
}
