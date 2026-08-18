use sardown_ast::BlockNode;
use sardown_style::Stylesheet;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures")).join(name)
}

fn code_block_texts(blocks: &[BlockNode]) -> Vec<String> {
    blocks
        .iter()
        .filter_map(|b| match b {
            BlockNode::CodeBlock { tokens, .. } => Some(tokens.iter().map(|t| t.text.as_str()).collect::<String>()),
            _ => None,
        })
        .collect()
}

#[test]
fn includes_a_whole_file_inside_a_fenced_code_block() {
    let blocks = sardown_book::load_book(&fixture("include-book"), &Stylesheet::default()).expect("load_book failed");
    let texts = code_block_texts(&blocks);
    assert!(
        texts.iter().any(|t| t.contains("fn main()") && t.contains("println!(\"hi\");")),
        "expected the whole included file's content in a code block, got: {texts:?}"
    );
}

#[test]
fn rustdoc_include_resolves_the_same_as_a_plain_include() {
    let blocks = sardown_book::load_book(&fixture("include-book"), &Stylesheet::default()).expect("load_book failed");
    let texts = code_block_texts(&blocks);
    let occurrences = texts.iter().filter(|t| t.contains("fn main()") && t.contains("println!(\"hi\");")).count();
    assert_eq!(occurrences, 2, "expected both {{{{#include}}}} and {{{{#rustdoc_include}}}} to resolve the whole file, got: {texts:?}");
}

#[test]
fn includes_only_the_requested_line_range() {
    let blocks = sardown_book::load_book(&fixture("include-book"), &Stylesheet::default()).expect("load_book failed");
    let texts = code_block_texts(&blocks);
    assert!(texts.iter().any(|t| t.trim() == "println!(\"hi\");"), "expected a code block containing only line 2 of snippet.rs, got: {texts:?}");
}

#[test]
fn a_sibling_of_src_include_within_the_book_root_is_allowed() {
    // Mirrors the official Rust Book's own layout: a shared snippet directory (there, `listings/`)
    // kept as a sibling of `src/` and referenced from chapters as `{{#include ../listings/...}}`.
    let blocks = sardown_book::load_book(&fixture("include-traversal-book/book"), &Stylesheet::default()).expect("load_book failed");
    let texts = code_block_texts(&blocks);
    assert!(texts.iter().any(|t| t.contains("SIBLING")), "expected a sibling-of-src include within the book root to succeed, got: {texts:?}");
}

#[test]
fn relative_traversal_outside_the_book_root_is_rejected() {
    let blocks = sardown_book::load_book(&fixture("include-traversal-book/book"), &Stylesheet::default()).expect("load_book failed");
    let texts = code_block_texts(&blocks);
    assert!(texts.iter().all(|t| !t.contains("SECRET")), "expected a relative-traversal include escaping the book root to be rejected, got: {texts:?}");
}

#[test]
fn an_absolute_include_path_is_rejected() {
    let blocks = sardown_book::load_book(&fixture("include-traversal-book/book"), &Stylesheet::default()).expect("load_book failed");
    let texts = code_block_texts(&blocks);
    assert!(texts.iter().all(|t| !t.contains("root:")), "expected an absolute include path to be rejected, not read from disk, got: {texts:?}");
}

#[test]
fn a_missing_include_target_is_dropped_with_a_warning_not_a_crash() {
    // The chapter still loads -- the unresolved directive line is simply omitted from the
    // code block that would have contained it, matching this project's graceful-degradation
    // convention for every other unresolvable reference (unknown fonts, external images, etc).
    let blocks = sardown_book::load_book(&fixture("include-book"), &Stylesheet::default()).expect("load_book failed");
    let texts = code_block_texts(&blocks);
    assert!(texts.iter().all(|t| !t.contains("{{#include")), "expected the unresolved directive to be dropped, not rendered literally, got: {texts:?}");
}
