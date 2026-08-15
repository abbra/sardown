use md2pdf_ast::{parse, BlockNode, InlineNode, LinkTarget};

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
