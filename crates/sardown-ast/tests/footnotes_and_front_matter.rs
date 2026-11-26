use sardown_ast::{parse, BlockNode, LinkTarget};

fn text(content: &[sardown_ast::InlineNode]) -> String {
    content.iter().map(|n| n.text.as_str()).collect()
}

#[test]
fn footnotes_are_numbered_by_citation_order_and_moved_to_the_end() {
    let blocks = parse("A[^b] and B[^a].\n\n[^a]: Alpha.\n\n[^b]: Beta.\n\n[^unused]: Dropped.\n");
    let BlockNode::Paragraph { content } = &blocks[0] else { panic!("{:?}", blocks[0]) };
    assert_eq!(text(content), "A¹ and B².");
    let Some(LinkTarget::Footnote(first)) = &content[1].link_target else { panic!("{:?}", content[1]) };
    let notes: Vec<_> = blocks[1..]
        .iter()
        .map(|b| match b {
            BlockNode::FootnoteDefinition { id, content } => (id.clone(), text(content)),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(notes.len(), 2);
    assert_eq!(&notes[0].0, first);
    assert_eq!(notes[0].1, "¹ Beta.");
    assert_eq!(notes[1].1, "² Alpha.");
}

#[test]
fn a_footnote_reference_with_no_definition_stays_literal_and_unlinked() {
    let blocks = parse("Real[^ok]. Ghost[^ghost] tail.\n\n[^ok]: Defined.\n");
    let BlockNode::Paragraph { content } = &blocks[0] else { panic!("{:?}", blocks[0]) };
    assert_eq!(text(content), "Real¹. Ghost[^ghost] tail.");
    let ghost = content.iter().find(|n| n.text.contains("ghost")).unwrap();
    assert!(ghost.link_target.is_none(), "{ghost:?}");
    assert_eq!(blocks.len(), 2);
    let BlockNode::FootnoteDefinition { id, content } = &blocks[1] else { panic!("{:?}", blocks[1]) };
    assert!(id.starts_with("footnote-"));
    assert_eq!(text(content), "¹ Defined.");
}

#[test]
fn front_matter_becomes_centered_lines_and_an_abstract_quote() {
    let blocks = parse("---\ntitle: T\nauthor: Jane\nabstract: |\n  Some *words*.\n---\n\nBody\n");
    let kinds: Vec<_> = blocks.iter().map(std::mem::discriminant).collect();
    assert_eq!(kinds.len(), 5, "{blocks:?}");
    let lines: Vec<String> = blocks[..3]
        .iter()
        .map(|b| match b {
            BlockNode::Centered { content } => text(content),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(lines, ["T", "Jane", "Abstract"]);
    assert!(matches!(&blocks[3], BlockNode::Blockquote { .. }));
    assert!(matches!(&blocks[4], BlockNode::Paragraph { .. }));
}

fn numbered(md: &str) -> Vec<String> {
    let mut style = sardown_style::Stylesheet::default();
    style.heading.number_from_level = 2;
    let mut slugs = sardown_ast::SlugGenerator::new();
    let mut next = 0;
    sardown_ast::parse_with_style(md, &mut slugs, &mut next, &style)
        .iter()
        .map(|b| match b {
            BlockNode::Heading { content, .. }
            | BlockNode::Paragraph { content }
            | BlockNode::Caption { content, .. }
            | BlockNode::Anchored { content, .. } => text(content),
            BlockNode::Math { source, .. } => source.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

#[test]
fn sections_captions_and_equations_are_numbered_and_cross_referenced() {
    let out = numbered(
        "## A {#sec:a}\n\n### B\n\nFigure: Cap. {#fig:x}\n\nTable: Tab.\n\n$$ x \\label{eq:x} $$\n\nSee @sec:a, @fig:x, @eq:x, @nobody and me@sec:a.\n",
    );
    assert_eq!(out[0], "1 A");
    assert_eq!(out[1], "1.1 B");
    assert_eq!(out[2], "Figure 1. Cap.");
    assert_eq!(out[3], "Table 1. Tab.");
    assert!(out[4].contains("\\tag{1}") && !out[4].contains("label"), "{}", out[4]);
    assert_eq!(out[5], "See Section 1, Figure 1, Eq. (1), @nobody and me@sec:a.");
}

#[test]
fn citations_are_numbered_by_first_use_and_listed_under_an_unnumbered_references_heading() {
    let md = "---\nreferences:\n  a: \"Alpha.\"\n  b: \"Beta.\"\n  c: \"Unused.\"\n---\n\n## Intro\n\nSee [@b] then [@a; @b], not [@zzz].\n";
    let texts = numbered(md);
    assert_eq!(texts, ["1 Intro", "See [1] then [1, 2], not [@zzz].", "References", "[1] Beta.", "[2] Alpha."]);
}
