use sardown_book::SummaryItem;

#[test]
fn parses_flat_chapter_list() {
    let md = "# Summary\n\n- [Chapter One](chapter1.md)\n- [Chapter Two](chapter2.md)\n";
    let summary = sardown_book::parse_summary(md);
    assert_eq!(summary.items.len(), 2);
    match &summary.items[0] {
        SummaryItem::Chapter { title, path, children } => {
            assert_eq!(title, "Chapter One");
            assert_eq!(path.as_deref(), Some(std::path::Path::new("chapter1.md")));
            assert!(children.is_empty());
        }
        other => panic!("expected Chapter, got {other:?}"),
    }
}

#[test]
fn parses_nested_chapters() {
    let md = "- [Parent](parent.md)\n  - [Child](child.md)\n";
    let summary = sardown_book::parse_summary(md);
    assert_eq!(summary.items.len(), 1);
    match &summary.items[0] {
        SummaryItem::Chapter { children, .. } => {
            assert_eq!(children.len(), 1);
            match &children[0] {
                SummaryItem::Chapter { title, .. } => assert_eq!(title, "Child"),
                other => panic!("expected Chapter, got {other:?}"),
            }
        }
        other => panic!("expected Chapter, got {other:?}"),
    }
}

#[test]
fn parses_part_titles_and_separators() {
    let md = "# Summary\n\n- [One](one.md)\n\n---\n\n# Part Two\n\n- [Two](two.md)\n";
    let summary = sardown_book::parse_summary(md);
    assert_eq!(summary.items.len(), 4, "expected Chapter(One), Separator, PartTitle, Chapter(Two), got {:?}", summary.items);
    assert!(matches!(&summary.items[0], SummaryItem::Chapter { title, .. } if title == "One"));
    assert_eq!(summary.items[1], SummaryItem::Separator);
    assert_eq!(summary.items[2], SummaryItem::PartTitle("Part Two".to_string()));
    assert!(matches!(&summary.items[3], SummaryItem::Chapter { title, .. } if title == "Two"));
}

#[test]
fn a_bare_link_before_the_first_list_item_is_a_prefix_chapter() {
    // Real mdBook lets an introduction/preface chapter appear as a bare link before the first
    // `-` list item, outside any list, so it isn't numbered like the rest of the chapters.
    let md = "# Summary\n\n[Introduction](introduction.md)\n\n- [Chapter One](chapter1.md)\n";
    let summary = sardown_book::parse_summary(md);
    assert_eq!(summary.items.len(), 2, "expected the prefix chapter plus one list chapter, got {:?}", summary.items);
    match &summary.items[0] {
        SummaryItem::Chapter { title, path, children } => {
            assert_eq!(title, "Introduction");
            assert_eq!(path.as_deref(), Some(std::path::Path::new("introduction.md")));
            assert!(children.is_empty());
        }
        other => panic!("expected the prefix chapter, got {other:?}"),
    }
    assert!(matches!(&summary.items[1], SummaryItem::Chapter { title, .. } if title == "Chapter One"));
}

#[test]
fn draft_chapters_have_no_path_but_are_still_walked_for_children() {
    let md = "- Draft One\n- [Draft Two]()\n- [Real](real.md)\n  - [Real Child](child.md)\n";
    let summary = sardown_book::parse_summary(md);
    assert_eq!(summary.items.len(), 3);
    assert!(matches!(&summary.items[0], SummaryItem::Chapter { path: None, .. }), "bare-text draft should have no path");
    assert!(matches!(&summary.items[1], SummaryItem::Chapter { path: None, .. }), "empty-link draft should have no path");
    match &summary.items[2] {
        SummaryItem::Chapter { path: Some(p), children, .. } => {
            assert_eq!(p, std::path::Path::new("real.md"));
            assert_eq!(children.len(), 1);
        }
        other => panic!("expected Chapter with a path and a child, got {other:?}"),
    }
}
