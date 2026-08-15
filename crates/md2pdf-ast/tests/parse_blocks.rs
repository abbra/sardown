use md2pdf_ast::{parse, BlockNode, ColumnAlignment, HighlightedToken, ImageSource, InlineNode, LinkTarget};

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

#[test]
fn parses_table_with_alignment() {
    let md = "| A | B |\n|:--|--:|\n| 1 | 2 |\n";
    let blocks = parse(md);
    match &blocks[0] {
        BlockNode::Table { headers, rows, alignments } => {
            assert_eq!(headers.iter().map(|cell| cell[0].text.clone()).collect::<Vec<_>>(), vec!["A", "B"]);
            assert_eq!(rows.len(), 1);
            assert_eq!(alignments, &vec![ColumnAlignment::Left, ColumnAlignment::Right]);
        }
        other => panic!("expected Table, got {other:?}"),
    }
}

#[test]
fn table_cell_with_mixed_inline_styling_keeps_all_its_text_in_one_cell() {
    // Regression test: a cell mixing plain text with a styled span (here, inline code) produces
    // more than one InlineNode. Flattening those into the row's flat list (instead of keeping
    // them grouped per cell) shifted every later cell in the row into the wrong column and
    // silently dropped whatever came after the last cell once `zip` ran out of columns.
    let md = "| Layer | Bytes |\n|---|---|\n| Outer `SignedData` headers plus a signature | ~555 B |\n";
    let blocks = parse(md);
    match &blocks[0] {
        BlockNode::Table { rows, .. } => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].len(), 2, "expected exactly 2 cells in the row, got {}", rows[0].len());
            let cell0_text: String = rows[0][0].iter().map(|n| n.text.as_str()).collect();
            assert_eq!(cell0_text, "Outer SignedData headers plus a signature");
            let cell1_text: String = rows[0][1].iter().map(|n| n.text.as_str()).collect();
            assert_eq!(cell1_text, "~555 B");
        }
        other => panic!("expected Table, got {other:?}"),
    }
}

#[test]
fn parses_external_and_relative_images() {
    let md = "![alt text](https://example.com/pic.png)\n\n![local](./pic.png)\n";
    let blocks = parse(md);
    match &blocks[0] {
        BlockNode::Image { alt, source, .. } => {
            assert_eq!(alt, "alt text");
            assert_eq!(source, &ImageSource::External("https://example.com/pic.png".to_string()));
        }
        other => panic!("expected Image, got {other:?}"),
    }
    match &blocks[1] {
        BlockNode::Image { source, .. } => {
            assert_eq!(source, &ImageSource::Embedded(std::path::PathBuf::from("./pic.png")));
        }
        other => panic!("expected Image, got {other:?}"),
    }
}

#[test]
fn parses_mermaid_fenced_code_block_as_diagram_not_code_block() {
    let md = "```mermaid\nflowchart TD\n    A --> B\n```\n";
    let blocks = parse(md);
    match &blocks[0] {
        BlockNode::MermaidDiagram { id, source } => {
            assert_eq!(id, "diagram-0");
            assert!(source.contains("flowchart TD"));
        }
        other => panic!("expected MermaidDiagram, got {other:?}"),
    }
}

#[test]
fn assigns_sequential_ids_to_multiple_diagrams() {
    let md = "```mermaid\nflowchart TD\n    A --> B\n```\n\n```mermaid\nflowchart TD\n    C --> D\n```\n";
    let blocks = parse(md);
    let ids: Vec<_> = blocks
        .iter()
        .map(|b| match b {
            BlockNode::MermaidDiagram { id, .. } => id.as_str(),
            other => panic!("expected MermaidDiagram, got {other:?}"),
        })
        .collect();
    assert_eq!(ids, vec!["diagram-0", "diagram-1"]);
}
