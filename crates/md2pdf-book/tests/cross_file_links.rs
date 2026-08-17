use md2pdf_ast::{BlockNode, InlineNode, LinkTarget};
use md2pdf_style::Stylesheet;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures")).join(name)
}

fn flatten_inline<'a>(blocks: &'a [BlockNode], out: &mut Vec<&'a InlineNode>) {
    for block in blocks {
        match block {
            BlockNode::Heading { content, .. } | BlockNode::Paragraph { content } => out.extend(content.iter()),
            BlockNode::Blockquote { content } => flatten_inline(content, out),
            BlockNode::List { items, .. } => {
                for item in items {
                    flatten_inline(item, out);
                }
            }
            BlockNode::Table { headers, rows, .. } => {
                for cell in headers {
                    out.extend(cell.iter());
                }
                for row in rows {
                    for cell in row {
                        out.extend(cell.iter());
                    }
                }
            }
            _ => {}
        }
    }
}

fn find_linked<'a>(nodes: &[&'a InlineNode], text: &str) -> &'a InlineNode {
    nodes.iter().find(|n| n.text == text && n.link_target.is_some()).unwrap_or_else(|| panic!("no linked inline node with text {text:?} found"))
}

#[test]
fn cross_file_link_with_fragment_resolves_through_the_post_merge_slug_map() {
    let blocks = md2pdf_book::load_book(&fixture("linked-book"), &Stylesheet::default()).expect("load_book failed");
    let mut nodes = Vec::new();
    flatten_inline(&blocks, &mut nodes);

    // Chapter1's link to "chapter2.md#overview" must resolve to chapter2's own (deduplicated)
    // heading -- "overview-1" -- not to chapter1's own "overview" heading, which a naive
    // same-slug-text lookup would incorrectly produce.
    let node = find_linked(&nodes, "Overview");
    assert_eq!(node.link_target, Some(LinkTarget::InternalAnchor("overview-1".to_string())));
}

#[test]
fn cross_file_link_without_fragment_resolves_to_chapter_start() {
    let blocks = md2pdf_book::load_book(&fixture("linked-book"), &Stylesheet::default()).expect("load_book failed");
    let mut nodes = Vec::new();
    flatten_inline(&blocks, &mut nodes);

    let node = find_linked(&nodes, "Chapter Two");
    assert_eq!(node.link_target, Some(LinkTarget::InternalAnchor("chapter-two".to_string())));
}

#[test]
fn unresolvable_or_non_chapter_links_are_left_inert_or_unchanged() {
    let blocks = md2pdf_book::load_book(&fixture("linked-book"), &Stylesheet::default()).expect("load_book failed");
    let mut nodes = Vec::new();
    flatten_inline(&blocks, &mut nodes);

    // Points at a real file that exists on disk but isn't listed in SUMMARY.md -- not a chapter,
    // so it's never classified as a cross-file link and is left exactly as parsed.
    let unrelated = nodes.iter().find(|n| n.text == "an unrelated file").expect("missing 'an unrelated file' node");
    assert_eq!(unrelated.link_target, Some(LinkTarget::ExternalUrl("not-a-chapter.md".to_string())));

    // Points at a real chapter but a fragment that doesn't match any heading -- classified as a
    // candidate, but unresolvable, so it falls back to inert (unlinked) text.
    let broken = nodes.iter().find(|n| n.text == "broken fragment").expect("missing 'broken fragment' node");
    assert_eq!(broken.link_target, None);

    // A true absolute URL is never touched by cross-file classification.
    let external = nodes.iter().find(|n| n.text == "example").expect("missing 'example' node");
    assert_eq!(external.link_target, Some(LinkTarget::ExternalUrl("https://example.com".to_string())));
}
