use md2pdf_ast::{parse, BlockNode, HighlightedToken, InlineNode, LinkTarget};

#[test]
fn parses_heading_and_paragraph_with_inline_styles_and_link() {
    let md = "# Hello World\n\nThis is **bold** and a [link](https://example.com).\n";
    let blocks = parse(md);

    assert_eq!(blocks.len(), 2);

    match &blocks[0] {
        BlockNode::Heading { level, id, content } => {
            assert_eq!(*level, 1);
            assert_eq!(id, "hello-world");
            assert_eq!(content.len(), 1);
            assert_eq!(content[0].text, "Hello World");
        }
        other => panic!("expected Heading, got {other:?}"),
    }

    match &blocks[1] {
        BlockNode::Paragraph { content } => {
            let bold_run = content.iter().find(|n: &&InlineNode| n.text == "bold").unwrap();
            assert!(bold_run.style.bold);

            let link_run = content.iter().find(|n: &&InlineNode| n.text == "link").unwrap();
            assert_eq!(link_run.link_target, Some(LinkTarget::ExternalUrl("https://example.com".to_string())));
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn internal_anchor_links_are_distinguished_from_external_urls() {
    let md = "[See intro](#intro)\n";
    let blocks = parse(md);
    match &blocks[0] {
        BlockNode::Paragraph { content } => {
            assert_eq!(content[0].link_target, Some(LinkTarget::InternalAnchor("intro".to_string())));
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn parses_code_block_with_language_and_raw_placeholder_token() {
    let md = "```rust\nfn main() {}\n```\n";
    let blocks = parse(md);
    match &blocks[0] {
        BlockNode::CodeBlock { language, tokens } => {
            assert_eq!(language.as_deref(), Some("rust"));
            assert_eq!(tokens, &vec![HighlightedToken { text: "fn main() {}\n".to_string(), color: [0, 0, 0] }]);
        }
        other => panic!("expected CodeBlock, got {other:?}"),
    }
}

#[test]
fn parses_blockquote_and_thematic_break() {
    let md = "> quoted text\n\n---\n";
    let blocks = parse(md);
    assert!(matches!(&blocks[0], BlockNode::Blockquote { content } if matches!(&content[0], BlockNode::Paragraph { .. })));
    assert!(matches!(&blocks[1], BlockNode::ThematicBreak));
}

#[test]
fn parses_nested_unordered_list() {
    let md = "- one\n- two\n  - nested\n";
    let blocks = parse(md);
    match &blocks[0] {
        BlockNode::List { ordered, items } => {
            assert!(!ordered);
            assert_eq!(items.len(), 2);
            // second item contains its own paragraph-less text plus a nested List block
            assert!(items[1].iter().any(|b| matches!(b, BlockNode::List { .. })));
        }
        other => panic!("expected List, got {other:?}"),
    }
}
