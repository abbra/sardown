use sardown_ast::{BlockNode, HighlightedToken};
use sardown_enrich::Highlighter;
use sardown_style::Stylesheet;

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
    let inner =
        BlockNode::CodeBlock { language: Some("rust".to_string()), tokens: vec![HighlightedToken { text: "let x = 1;\n".to_string(), color: [0, 0, 0] }] };
    let ast = vec![BlockNode::Blockquote { content: vec![inner.clone()] }, BlockNode::List { ordered: false, start: None, items: vec![vec![inner]] }];
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

#[test]
fn with_style_uses_the_configured_syntax_theme() {
    // Can't inspect which theme loaded directly, but two different themes highlighting the same
    // source must produce different colors -- proving the configured name actually took effect.
    let mut style_a = Stylesheet::default();
    style_a.code_block.syntax_theme = "InspiredGitHub".to_string();
    let mut style_b = Stylesheet::default();
    style_b.code_block.syntax_theme = "base16-ocean.dark".to_string();

    let ast = vec![BlockNode::CodeBlock {
        language: Some("rust".to_string()),
        tokens: vec![HighlightedToken { text: "fn main() {}\n".to_string(), color: [0, 0, 0] }],
    }];

    let result_a = Highlighter::with_style(&style_a).highlight(ast.clone());
    let result_b = Highlighter::with_style(&style_b).highlight(ast);

    let colors_of = |result: &[BlockNode]| match &result[0] {
        BlockNode::CodeBlock { tokens, .. } => tokens.iter().map(|t| t.color).collect::<Vec<_>>(),
        other => panic!("expected CodeBlock, got {other:?}"),
    };
    assert_ne!(colors_of(&result_a), colors_of(&result_b), "expected different themes to produce different highlight colors");
}

#[test]
fn with_style_falls_back_to_inspired_github_for_an_unknown_theme() {
    let mut style = Stylesheet::default();
    style.code_block.syntax_theme = "not-a-real-theme".to_string();
    let ast = vec![BlockNode::CodeBlock {
        language: Some("rust".to_string()),
        tokens: vec![HighlightedToken { text: "fn main() {}\n".to_string(), color: [0, 0, 0] }],
    }];
    let result = Highlighter::with_style(&style).highlight(ast);
    match &result[0] {
        BlockNode::CodeBlock { tokens, .. } => assert!(tokens.len() > 1, "expected real highlighting despite the invalid theme name"),
        other => panic!("expected CodeBlock, got {other:?}"),
    }
}

#[test]
fn new_matches_with_style_using_stylesheet_defaults() {
    let ast = vec![BlockNode::CodeBlock {
        language: Some("rust".to_string()),
        tokens: vec![HighlightedToken { text: "fn main() {}\n".to_string(), color: [0, 0, 0] }],
    }];
    let result_a = Highlighter::new().highlight(ast.clone());
    let result_b = Highlighter::with_style(&Stylesheet::default()).highlight(ast);
    assert_eq!(result_a, result_b);
}

#[test]
fn recurses_into_columns() {
    let ast = vec![BlockNode::Columns(vec![vec![BlockNode::CodeBlock {
        language: Some("rust".to_string()),
        tokens: vec![HighlightedToken { text: "fn main() {}\n".to_string(), color: [0, 0, 0] }],
    }]])];
    let result = Highlighter::new().highlight(ast);
    let BlockNode::Columns(columns) = &result[0] else { panic!("expected Columns") };
    match &columns[0][0] {
        BlockNode::CodeBlock { tokens, .. } => assert!(tokens.len() > 1, "expected the code block inside the column to be highlighted"),
        other => panic!("expected CodeBlock, got {other:?}"),
    }
}
