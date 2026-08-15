use md2pdf_ast::{BlockNode, InlineNode, LinkTarget};
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

/// Interprets `SUMMARY.md`'s structure by reusing `md2pdf_ast::parse` (no new markdown-parsing
/// code) and walking the resulting blocks: mdBook's indentation-based chapter nesting *is*
/// markdown list nesting, so pulldown-cmark's own nested-list parsing handles it for free.
pub fn parse_summary(markdown: &str) -> BookSummary {
    let blocks = md2pdf_ast::parse(markdown);
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
                title = inline_text(content);
                path = content.iter().find_map(|n| match &n.link_target {
                    Some(LinkTarget::ExternalUrl(url)) if !url.is_empty() => Some(PathBuf::from(url)),
                    _ => None,
                });
            }
            BlockNode::List { items: nested, .. } => {
                children.extend(nested.iter().map(|item| summary_item_from_list_item(item)));
            }
            _ => {}
        }
    }
    SummaryItem::Chapter { title, path, children }
}

fn inline_text(content: &[InlineNode]) -> String {
    content.iter().map(|n| n.text.as_str()).collect()
}
