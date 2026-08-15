use md2pdf_ast::{BlockNode, HighlightedToken};
use md2pdf_enrich::Highlighter;

#[test]
fn highlights_a_rust_code_block_with_more_than_one_color() {
    let ast = vec![BlockNode::CodeBlock {
        language: Some("rust".to_string()),
        tokens: vec![HighlightedToken { text: "fn main() {}\n".to_string(), color: [0, 0, 0] }],
    }];

    let highlighter = Highlighter::new();
    let result = highlighter.highlight(ast);

    match &result[0] {
        BlockNode::CodeBlock { tokens, .. } => {
            assert!(tokens.len() > 1, "expected multiple tokens from real highlighting, got {}", tokens.len());
            let colors: std::collections::HashSet<_> = tokens.iter().map(|t| t.color).collect();
            assert!(colors.len() > 1, "expected more than one distinct color across tokens");
        }
        other => panic!("expected CodeBlock, got {other:?}"),
    }
}

#[test]
fn leaves_non_code_blocks_unchanged() {
    let ast = vec![BlockNode::ThematicBreak];
    let highlighter = Highlighter::new();
    let result = highlighter.highlight(ast);
    assert!(matches!(result[0], BlockNode::ThematicBreak));
}

#[test]
fn recurses_into_blockquotes_and_list_items() {
    let inner = BlockNode::CodeBlock {
        language: Some("rust".to_string()),
        tokens: vec![HighlightedToken { text: "let x = 1;\n".to_string(), color: [0, 0, 0] }],
    };
    let ast = vec![
        BlockNode::Blockquote { content: vec![inner.clone()] },
        BlockNode::List { ordered: false, items: vec![vec![inner]] },
    ];
    let highlighter = Highlighter::new();
    let result = highlighter.highlight(ast);

    let assert_highlighted = |block: &BlockNode| match block {
        BlockNode::CodeBlock { tokens, .. } => assert!(tokens.len() > 1),
        other => panic!("expected CodeBlock, got {other:?}"),
    };
    match &result[0] {
        BlockNode::Blockquote { content } => assert_highlighted(&content[0]),
        other => panic!("expected Blockquote, got {other:?}"),
    }
    match &result[1] {
        BlockNode::List { items, .. } => assert_highlighted(&items[0][0]),
        other => panic!("expected List, got {other:?}"),
    }
}
