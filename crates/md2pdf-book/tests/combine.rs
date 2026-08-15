use md2pdf_ast::{BlockNode, ImageSource};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures")).join(name)
}

#[test]
fn combines_chapters_in_summary_order_with_page_breaks_and_synthesized_headings() {
    let blocks = md2pdf_book::load_book(&fixture("minimal-book")).expect("load_book failed");

    let page_break_count = blocks.iter().filter(|b| matches!(b, BlockNode::PageBreak)).count();
    assert_eq!(page_break_count, 2, "expected one PageBreak per chapter, got blocks: {blocks:?}");

    // Chapter One already has its own "# Chapter One" heading in its source file.
    assert!(matches!(&blocks[0], BlockNode::PageBreak));
    assert!(matches!(&blocks[1], BlockNode::Heading { content, .. } if content[0].text == "Chapter One"));

    // Chapter Two has no top-level heading in its source file -- one must be synthesized from
    // its SUMMARY.md title.
    let chapter_two_heading_index = blocks.iter().position(|b| matches!(b, BlockNode::Heading { content, .. } if content[0].text == "Chapter Two"));
    assert!(chapter_two_heading_index.is_some(), "expected a heading synthesized from Chapter Two's SUMMARY.md title");
    assert!(
        matches!(&blocks[chapter_two_heading_index.unwrap() - 1], BlockNode::PageBreak),
        "expected the synthesized heading to be immediately preceded by a PageBreak"
    );
}

#[test]
fn works_without_a_book_toml_and_with_nested_chapters() {
    let blocks = md2pdf_book::load_book(&fixture("nested-book")).expect("load_book failed");
    assert!(!blocks.is_empty(), "expected chapters to load even with no book.toml present");
    let heading_texts: Vec<_> = blocks
        .iter()
        .filter_map(|b| match b {
            BlockNode::Heading { content, .. } => Some(content[0].text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(heading_texts, vec!["Intro", "Child"], "expected both the parent and nested chapter to be included, in order");
}

#[test]
fn resolves_each_chapters_images_relative_to_its_own_directory() {
    let blocks = md2pdf_book::load_book(&fixture("nested-book")).expect("load_book failed");
    let image_paths: Vec<_> = blocks
        .iter()
        .filter_map(|b| match b {
            BlockNode::Image { source: ImageSource::Embedded(path), .. } => Some(path.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(image_paths.len(), 2, "expected one image per chapter, got {image_paths:?}");
    for path in &image_paths {
        assert!(path.is_absolute(), "expected image paths to be rewritten as absolute, got {}", path.display());
        assert!(path.exists(), "expected the resolved image path to actually exist on disk: {}", path.display());
    }
}

#[test]
fn missing_summary_md_is_an_error() {
    let result = md2pdf_book::load_book(&fixture("does-not-exist"));
    assert!(result.is_err(), "expected a missing book root/SUMMARY.md to be a real error, not silently empty output");
}
