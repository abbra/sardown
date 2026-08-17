use sardown_ast::{BlockNode, InlineNode, LinkTarget};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum SummaryItem {
    Chapter { title: String, path: Option<PathBuf>, children: Vec<SummaryItem> },
    PartTitle(String),
    Separator,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BookSummary {
    pub items: Vec<SummaryItem>,
}

/// Interprets `SUMMARY.md`'s structure by reusing `sardown_ast::parse` (no new markdown-parsing
/// code) and walking the resulting blocks: mdBook's indentation-based chapter nesting *is*
/// markdown list nesting, so pulldown-cmark's own nested-list parsing handles it for free.
pub fn parse_summary(markdown: &str) -> BookSummary {
    let blocks = sardown_ast::parse(markdown);
    let mut items = Vec::new();
    let mut seen_first_heading = false;
    for block in &blocks {
        match block {
            BlockNode::Heading { content, .. } => {
                // The *first* heading is SUMMARY.md's own title (e.g. "# Summary"), not a part
                // title -- mdBook doesn't render it as a sidebar grouping label.
                if !seen_first_heading {
                    seen_first_heading = true;
                    continue;
                }
                items.push(SummaryItem::PartTitle(inline_text(content)));
            }
            BlockNode::ThematicBreak => items.push(SummaryItem::Separator),
            BlockNode::List { items: list_items, .. } => {
                items.extend(list_items.iter().map(|item| summary_item_from_list_item(item)));
            }
            // A bare link paragraph outside any list -- real mdBook's "prefix chapter"
            // convention, used for an introduction/preface that shouldn't be numbered like the
            // rest of the chapters. Treated the same as a list-item chapter with no children;
            // previously fell through to the catch-all below and was silently dropped entirely.
            BlockNode::Paragraph { content } => {
                let (title, path) = chapter_title_and_path(content);
                items.push(SummaryItem::Chapter { title, path, children: Vec::new() });
            }
            _ => {}
        }
    }
    BookSummary { items }
}

fn summary_item_from_list_item(blocks: &[BlockNode]) -> SummaryItem {
    let mut title = String::new();
    let mut path = None;
    let mut children = Vec::new();
    for block in blocks {
        match block {
            BlockNode::Paragraph { content } => {
                (title, path) = chapter_title_and_path(content);
            }
            BlockNode::List { items: nested, .. } => {
                children.extend(nested.iter().map(|item| summary_item_from_list_item(item)));
            }
            _ => {}
        }
    }
    SummaryItem::Chapter { title, path, children }
}

fn chapter_title_and_path(content: &[InlineNode]) -> (String, Option<PathBuf>) {
    let title = inline_text(content);
    let path = content.iter().find_map(|n| match &n.link_target {
        Some(LinkTarget::ExternalUrl(url)) if !url.is_empty() => Some(PathBuf::from(url)),
        _ => None,
    });
    (title, path)
}

fn inline_text(content: &[InlineNode]) -> String {
    content.iter().map(|n| n.text.as_str()).collect()
}
