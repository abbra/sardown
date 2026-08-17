use md2pdf_ast::{generate_heading_id, BlockNode, InlineNode, LinkTarget};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::summary::SummaryItem;

/// The canonicalized, absolute path of every chapter file listed anywhere in `SUMMARY.md`
/// (recursively, including nested chapters). A relative link only becomes a `CrossFileAnchor`
/// if it resolves to a path in this set -- otherwise it's left as a plain `ExternalUrl`.
pub(crate) fn known_chapter_files(items: &[SummaryItem], src_dir: &Path) -> HashSet<PathBuf> {
    let mut files = HashSet::new();
    collect(items, src_dir, &mut files);
    files
}

fn collect(items: &[SummaryItem], src_dir: &Path, files: &mut HashSet<PathBuf>) {
    for item in items {
        if let SummaryItem::Chapter { path, children, .. } = item {
            if let Some(rel_path) = path {
                if let Ok(canonical) = std::fs::canonicalize(src_dir.join(rel_path)) {
                    files.insert(canonical);
                }
            }
            collect(children, src_dir, files);
        }
    }
}

fn is_absolute_url(url: &str) -> bool {
    url.contains("://") || url.starts_with("mailto:")
}

fn split_fragment(url: &str) -> (&str, Option<&str>) {
    match url.split_once('#') {
        Some((path, frag)) => (path, Some(frag)),
        None => (url, None),
    }
}

/// Rewrites every `ExternalUrl` in `blocks` that resolves (relative to `chapter_dir`) to a known
/// chapter file into a `CrossFileAnchor`. A link whose target file isn't a listed chapter, or
/// that's already absolute (`scheme://`, `mailto:`), is left untouched.
pub(crate) fn classify_links(blocks: &mut [BlockNode], chapter_dir: &Path, known_chapter_files: &HashSet<PathBuf>) {
    for block in blocks {
        match block {
            BlockNode::Heading { content, .. } | BlockNode::Paragraph { content } => {
                classify_inline(content, chapter_dir, known_chapter_files);
            }
            BlockNode::Blockquote { content } => classify_links(content, chapter_dir, known_chapter_files),
            BlockNode::List { items, .. } => {
                for item in items {
                    classify_links(item, chapter_dir, known_chapter_files);
                }
            }
            BlockNode::Table { headers, rows, .. } => {
                for cell in headers {
                    classify_inline(cell, chapter_dir, known_chapter_files);
                }
                for row in rows {
                    for cell in row {
                        classify_inline(cell, chapter_dir, known_chapter_files);
                    }
                }
            }
            BlockNode::Columns(columns) => {
                for column in columns {
                    classify_links(column, chapter_dir, known_chapter_files);
                }
            }
            _ => {}
        }
    }
}

fn classify_inline(content: &mut [InlineNode], chapter_dir: &Path, known_chapter_files: &HashSet<PathBuf>) {
    for node in content {
        if let Some(LinkTarget::ExternalUrl(url)) = &node.link_target {
            if is_absolute_url(url) {
                continue;
            }
            let (file_part, fragment) = split_fragment(url);
            if file_part.is_empty() {
                continue;
            }
            if let Ok(canonical) = std::fs::canonicalize(chapter_dir.join(file_part)) {
                if known_chapter_files.contains(&canonical) {
                    node.link_target = Some(LinkTarget::CrossFileAnchor { file: canonical, fragment: fragment.map(str::to_string) });
                }
            }
        }
    }
}

/// Records, for every heading in `blocks`, `(chapter_path, pre-merge slug) -> actual final slug`
/// -- the pre-merge slug is what a same-book link author would have written, computed with the
/// pure `generate_heading_id` (no cross-chapter deduplication); the final slug is whatever the
/// shared `SlugGenerator` actually assigned once merged, which may differ if another chapter
/// already claimed the same text.
pub(crate) fn record_heading_slugs(blocks: &[BlockNode], chapter_path: &Path, slug_map: &mut HashMap<(PathBuf, String), String>) {
    for block in blocks {
        match block {
            BlockNode::Heading { id, content, .. } => {
                let text: String = content.iter().map(|n| n.text.as_str()).collect();
                let original = generate_heading_id(&text);
                slug_map.insert((chapter_path.to_path_buf(), original), id.clone());
            }
            BlockNode::Blockquote { content } => record_heading_slugs(content, chapter_path, slug_map),
            BlockNode::List { items, .. } => {
                for item in items {
                    record_heading_slugs(item, chapter_path, slug_map);
                }
            }
            BlockNode::Columns(columns) => {
                for column in columns {
                    record_heading_slugs(column, chapter_path, slug_map);
                }
            }
            _ => {}
        }
    }
}

/// Rewrites every `CrossFileAnchor` in the fully-combined `blocks` into an `InternalAnchor`
/// using `slug_map` (fragment links) or `chapter_start_map` (whole-file links), or drops it to
/// inert (unlinked) text if it can't be resolved.
pub(crate) fn resolve_links(blocks: &mut [BlockNode], slug_map: &HashMap<(PathBuf, String), String>, chapter_start_map: &HashMap<PathBuf, String>) {
    for block in blocks {
        match block {
            BlockNode::Heading { content, .. } | BlockNode::Paragraph { content } => {
                resolve_inline(content, slug_map, chapter_start_map);
            }
            BlockNode::Blockquote { content } => resolve_links(content, slug_map, chapter_start_map),
            BlockNode::List { items, .. } => {
                for item in items {
                    resolve_links(item, slug_map, chapter_start_map);
                }
            }
            BlockNode::Table { headers, rows, .. } => {
                for cell in headers {
                    resolve_inline(cell, slug_map, chapter_start_map);
                }
                for row in rows {
                    for cell in row {
                        resolve_inline(cell, slug_map, chapter_start_map);
                    }
                }
            }
            BlockNode::Columns(columns) => {
                for column in columns {
                    resolve_links(column, slug_map, chapter_start_map);
                }
            }
            _ => {}
        }
    }
}

fn resolve_inline(content: &mut [InlineNode], slug_map: &HashMap<(PathBuf, String), String>, chapter_start_map: &HashMap<PathBuf, String>) {
    for node in content {
        if let Some(LinkTarget::CrossFileAnchor { file, fragment }) = &node.link_target {
            let resolved = match fragment {
                Some(frag) => slug_map.get(&(file.clone(), frag.clone())).cloned(),
                None => chapter_start_map.get(file).cloned(),
            };
            node.link_target = resolved.map(LinkTarget::InternalAnchor);
        }
    }
}
