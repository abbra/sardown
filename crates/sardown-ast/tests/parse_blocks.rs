use sardown_ast::{parse, BlockNode, ColumnAlignment, HighlightedToken, ImageSource, InlineNode, LinkTarget};

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
        BlockNode::List { ordered, items, .. } => {
            assert!(!ordered);
            assert_eq!(items.len(), 2);
            let item0_text: String = items[0]
                .iter()
                .filter_map(|b| match b {
                    BlockNode::Paragraph { content } => Some(content.iter().map(|n| n.text.as_str()).collect::<String>()),
                    _ => None,
                })
                .collect();
            assert_eq!(item0_text, "one", "first item's own text should be present, not dropped");
            let item1_text: String = items[1]
                .iter()
                .filter_map(|b| match b {
                    BlockNode::Paragraph { content } => Some(content.iter().map(|n| n.text.as_str()).collect::<String>()),
                    _ => None,
                })
                .collect();
            assert_eq!(item1_text, "two", "second item's own text should be present alongside its nested list");
            assert!(items[1].iter().any(|b| matches!(b, BlockNode::List { .. })), "second item should still have its nested List block");
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn unordered_list_has_no_start_number() {
    let blocks = parse("- one\n- two\n");
    match &blocks[0] {
        BlockNode::List { ordered, start, .. } => {
            assert!(!ordered);
            assert_eq!(*start, None);
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn ordered_list_defaults_to_starting_at_one() {
    let blocks = parse("1. one\n2. two\n");
    match &blocks[0] {
        BlockNode::List { ordered, start, .. } => {
            assert!(ordered);
            assert_eq!(*start, Some(1));
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn ordered_list_captures_a_non_default_start_number() {
    // CommonMark honors the literal number the first item is written with -- "5." should
    // render starting at 5, not silently reset to 1.
    let blocks = parse("5. fifth\n6. sixth\n");
    match &blocks[0] {
        BlockNode::List { ordered, start, .. } => {
            assert!(ordered);
            assert_eq!(*start, Some(5));
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn tight_list_items_are_not_dropped() {
    // Regression test: a "tight" list (no blank lines between items -- the overwhelmingly
    // common way lists are actually written) doesn't get pulldown-cmark's Tag::Paragraph
    // wrapper around each item's inline content; it emits bare Text/Strong/Emphasis/Link
    // events directly inside Tag::Item. The block-level parser only recognized events wrapped
    // in a known block tag, so every tight list item's content was silently dropped entirely.
    let md = "- one\n- two\n- three\n";
    let blocks = parse(md);
    match &blocks[0] {
        BlockNode::List { items, .. } => {
            assert_eq!(items.len(), 3);
            for (item, expected) in items.iter().zip(["one", "two", "three"]) {
                let text: String = item
                    .iter()
                    .filter_map(|b| match b {
                        BlockNode::Paragraph { content } => Some(content.iter().map(|n| n.text.as_str()).collect::<String>()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(text, expected, "tight list item content was dropped");
            }
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn tight_list_item_preserves_inline_styling_and_links() {
    let md = "- plain **bold** [a link](https://example.com/x) more\n";
    let blocks = parse(md);
    match &blocks[0] {
        BlockNode::List { items, .. } => {
            assert_eq!(items.len(), 1);
            let content = items[0].iter().find_map(|b| match b {
                BlockNode::Paragraph { content } => Some(content),
                _ => None,
            });
            let content = content.expect("expected the tight item's content to survive as a Paragraph");
            let bold_run = content.iter().find(|n| n.text == "bold").expect("expected a 'bold' run");
            assert!(bold_run.style.bold, "expected the bold run to carry bold styling");
            let link_run = content.iter().find(|n| n.text == "a link").expect("expected 'a link' run");
            assert_eq!(link_run.link_target, Some(LinkTarget::ExternalUrl("https://example.com/x".to_string())));
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
fn table_cell_text_is_smaller_than_default_body_text() {
    // Tables read as visually cramped at full body size once cell padding was fixed to give
    // borders proper breathing room; a slightly smaller size reads better in a narrow cell.
    let md = "| A |\n| --- |\n| x |\n";
    let blocks = parse(md);
    match &blocks[0] {
        BlockNode::Table { headers, rows, .. } => {
            assert!(headers[0][0].style.size < 12.0, "expected table header text smaller than default body size (12.0)");
            assert!(rows[0][0][0].style.size < 12.0, "expected table row text smaller than default body size (12.0)");
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
fn parses_a_base64_data_uri_image_as_a_data_uri_source_not_an_embedded_path() {
    let md = "![alt text](data:image/png;base64,iVBORw0KGgo=)\n";
    let blocks = parse(md);
    match &blocks[0] {
        BlockNode::Image { alt, source, .. } => {
            assert_eq!(alt, "alt text");
            assert_eq!(source, &ImageSource::DataUri("data:image/png;base64,iVBORw0KGgo=".to_string()));
        }
        other => panic!("expected Image, got {other:?}"),
    }
}

#[test]
fn parses_mermaid_fenced_code_block_as_diagram_not_code_block() {
    let md = "```mermaid\nflowchart TD\n    A --> B\n```\n";
    let blocks = parse(md);
    match &blocks[0] {
        BlockNode::MermaidDiagram { id, source, .. } => {
            assert_eq!(id, "diagram-0");
            assert!(source.contains("flowchart TD"));
        }
        other => panic!("expected MermaidDiagram, got {other:?}"),
    }
}

#[test]
fn mermaid_diagram_records_its_source_line_and_column() {
    // So a failed-to-render warning can point back at the real source location instead of just
    // an opaque synthetic id like "diagram-0".
    let md = "Intro text.\n\n```mermaid\nflowchart TD\n    A --> B\n```\n";
    let blocks = parse(md);
    match &blocks[1] {
        BlockNode::MermaidDiagram { line, column, file, .. } => {
            assert_eq!(*line, 3, "expected the line the diagram's opening fence starts on");
            assert_eq!(*column, 1);
            assert_eq!(*file, None, "parse() has no file context to attach on its own");
        }
        other => panic!("expected MermaidDiagram, got {other:?}"),
    }
}

#[test]
fn tag_diagram_origins_sets_file_on_every_diagram_recursively() {
    let md = "> ```mermaid\n> flowchart TD\n>     A --> B\n> ```\n";
    let mut blocks = parse(md);
    sardown_ast::tag_diagram_origins(&mut blocks, std::path::Path::new("chapter1.md"));
    match &blocks[0] {
        BlockNode::Blockquote { content } => match &content[0] {
            BlockNode::MermaidDiagram { file, .. } => {
                assert_eq!(file.as_deref(), Some(std::path::Path::new("chapter1.md")));
            }
            other => panic!("expected MermaidDiagram, got {other:?}"),
        },
        other => panic!("expected Blockquote, got {other:?}"),
    }
}

#[test]
fn tag_diagram_origins_recurses_into_columns() {
    let mut blocks = vec![BlockNode::Columns(vec![vec![BlockNode::MermaidDiagram {
        id: "d0".to_string(),
        source: "flowchart TD\n    A --> B".to_string(),
        line: 1,
        column: 1,
        file: None,
    }]])];
    sardown_ast::tag_diagram_origins(&mut blocks, std::path::Path::new("chapter1.md"));
    let BlockNode::Columns(columns) = &blocks[0] else { panic!("expected Columns") };
    let BlockNode::MermaidDiagram { file, .. } = &columns[0][0] else { panic!("expected MermaidDiagram") };
    assert_eq!(file.as_deref(), Some(std::path::Path::new("chapter1.md")));
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

#[test]
fn parse_with_slugs_shares_slug_generator_across_calls() {
    let mut slugs = sardown_ast::SlugGenerator::new();
    let mut next_diagram_id = 0usize;
    let first = sardown_ast::parse_with_slugs("# Overview\n", &mut slugs, &mut next_diagram_id);
    let second = sardown_ast::parse_with_slugs("# Overview\n", &mut slugs, &mut next_diagram_id);

    let id_of = |blocks: &[BlockNode]| match &blocks[0] {
        BlockNode::Heading { id, .. } => id.clone(),
        other => panic!("expected Heading, got {other:?}"),
    };
    assert_eq!(id_of(&first), "overview");
    assert_eq!(id_of(&second), "overview-1", "second call's heading should be deduplicated against the first");
}

#[test]
fn parse_with_slugs_shares_diagram_id_counter_across_calls() {
    let mut slugs = sardown_ast::SlugGenerator::new();
    let mut next_diagram_id = 0usize;
    let md = "```mermaid\nflowchart TD\n    A --> B\n```\n";
    let first = sardown_ast::parse_with_slugs(md, &mut slugs, &mut next_diagram_id);
    let second = sardown_ast::parse_with_slugs(md, &mut slugs, &mut next_diagram_id);

    let id_of = |blocks: &[BlockNode]| match &blocks[0] {
        BlockNode::MermaidDiagram { id, .. } => id.clone(),
        other => panic!("expected MermaidDiagram, got {other:?}"),
    };
    assert_eq!(id_of(&first), "diagram-0");
    assert_eq!(id_of(&second), "diagram-1", "second call's diagram id should not collide with the first");
}

#[test]
fn strikethrough_text_sets_the_strikethrough_style_flag() {
    let ast = parse("Some ~~struck~~ text.\n");
    match &ast[0] {
        BlockNode::Paragraph { content } => {
            let plain = content.iter().find(|n| n.text.contains("Some")).expect("missing plain text run");
            let struck = content.iter().find(|n| n.text == "struck").expect("missing struck-through run");
            assert!(!plain.style.strikethrough, "plain text should not be marked strikethrough");
            assert!(struck.style.strikethrough, "expected the ~~struck~~ run to be marked strikethrough");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn inline_code_spans_use_a_monospace_font_distinct_from_surrounding_text() {
    let ast = parse("Some `inline code` here.\n");
    match &ast[0] {
        BlockNode::Paragraph { content } => {
            let plain = content.iter().find(|n| n.text.contains("Some")).expect("missing plain text run");
            let code = content.iter().find(|n| n.text == "inline code").expect("missing inline code run");
            assert_ne!(code.style.font_family, plain.style.font_family, "expected inline code to use a distinct font family from surrounding text");
            assert_eq!(code.style.font_family.as_ref(), "monospace");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn task_list_items_render_a_checkbox_glyph_instead_of_literal_brackets() {
    let md = "- [ ] Unchecked\n- [x] Checked\n";
    let blocks = parse(md);
    match &blocks[0] {
        BlockNode::List { items, .. } => {
            assert_eq!(items.len(), 2);
            let text_of = |item: &[BlockNode]| -> String {
                item.iter()
                    .filter_map(|b| match b {
                        BlockNode::Paragraph { content } => Some(content.iter().map(|n| n.text.as_str()).collect::<String>()),
                        _ => None,
                    })
                    .collect()
            };
            let unchecked = text_of(&items[0]);
            let checked = text_of(&items[1]);
            assert!(unchecked.contains('\u{2610}'), "expected an unchecked box glyph, got {unchecked:?}");
            assert!(unchecked.contains("Unchecked"));
            assert!(checked.contains('\u{2611}'), "expected a checked box glyph, got {checked:?}");
            assert!(checked.contains("Checked"));
            assert!(!unchecked.contains("[ ]"), "expected the literal \"[ ]\" to be replaced, not kept alongside the glyph");
            assert!(!checked.contains("[x]"), "expected the literal \"[x]\" to be replaced, not kept alongside the glyph");
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn heading_style_for_level_matches_parse_generated_sizes() {
    let ast = parse("# H1\n\n## H2\n");
    let size_of = |block: &BlockNode| match block {
        BlockNode::Heading { content, .. } => content[0].style.size,
        other => panic!("expected Heading, got {other:?}"),
    };
    assert_eq!(sardown_ast::heading_style_for_level(1).size, size_of(&ast[0]));
    assert_eq!(sardown_ast::heading_style_for_level(2).size, size_of(&ast[1]));
    assert!(!sardown_ast::heading_style_for_level(1).bold);
    assert!(!sardown_ast::heading_style_for_level(1).italic);
}
