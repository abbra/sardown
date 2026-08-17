use std::path::{Path, PathBuf};

struct Directive {
    path: PathBuf,
    start: Option<usize>,
    end: Option<usize>,
}

/// Resolves `{{#include path[:start[:end]]}}` directives in `text`, splicing in the referenced
/// file's content before the chapter is parsed as markdown. A directive must be the entire
/// (trimmed) content of its own line -- the overwhelmingly common real-world usage, and simple
/// to detect without a regex dependency. A line starting with a backslash (`\{{#include ...}}`,
/// this project's own docs use this to talk *about* the syntax without triggering it) is left
/// untouched for pulldown-cmark's own backslash-escape handling to render literally downstream.
///
/// `path` comes directly from Markdown authored by whoever wrote the book being rendered, so
/// (mirroring `md2pdf-layout::image`'s identical guard for embedded images) it's resolved against
/// `chapter_dir` and rejected unless the result stays within `src_dir` -- the whole book's source
/// tree, not just this one chapter's directory, so a shared snippet in a sibling chapter's folder
/// still works while `../../../../etc/passwd` does not.
///
/// Only whole-file and line-range forms are supported -- mdBook's anchor-based
/// (`// ANCHOR: name`) ranges are not. A directive whose target can't be read or resolves outside
/// `src_dir` is dropped with a warning, matching this project's graceful-degradation convention
/// for every other unresolvable reference, rather than rendered literally or left to break the
/// surrounding code fence.
pub fn resolve_includes(text: &str, chapter_dir: &Path, src_dir: &Path) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        match parse_directive(line.trim()) {
            Some(directive) => match resolve_within_src(src_dir, chapter_dir, &directive.path) {
                Ok(target) => match std::fs::read_to_string(&target) {
                    Ok(contents) => out.push_str(&slice_lines(&contents, directive.start, directive.end)),
                    Err(e) => eprintln!("warning: failed to include {}: {e}", target.display()),
                },
                Err(e) => eprintln!("warning: refusing to include {}: {e}", directive.path.display()),
            },
            None => out.push_str(line),
        }
    }
    out
}

/// Rejects an absolute `path` outright (`Path::join` would discard `chapter_dir` entirely and use
/// it verbatim), then canonicalizes the joined result and requires it to stay within `src_dir` --
/// canonicalizing resolves `..` and symlinks via the OS, so this can't be fooled by a symlink that
/// lexically looks contained but points outside `src_dir`.
fn resolve_within_src(src_dir: &Path, chapter_dir: &Path, path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Err(format!("absolute include paths are not allowed: {}", path.display()));
    }
    let canonical_src = src_dir.canonicalize().map_err(|e| format!("cannot resolve book source directory {}: {e}", src_dir.display()))?;
    let candidate = chapter_dir.join(path);
    let canonical_candidate = candidate.canonicalize().map_err(|e| format!("cannot resolve path: {e}"))?;
    if canonical_candidate.starts_with(&canonical_src) {
        Ok(canonical_candidate)
    } else {
        Err(format!("path escapes book source directory {}", canonical_src.display()))
    }
}

fn parse_directive(trimmed_line: &str) -> Option<Directive> {
    let body = trimmed_line.strip_prefix("{{#include")?.strip_suffix("}}")?;
    let mut segments = body.trim().splitn(3, ':');
    let path = segments.next()?.trim();
    if path.is_empty() {
        return None;
    }
    let rest: Vec<&str> = segments.collect();
    let (start, end) = match rest.as_slice() {
        [] => (None, None),
        [single] => (parse_line_number(single), parse_line_number(single)),
        [start, end] => (parse_line_number(start), parse_line_number(end)),
        _ => unreachable!("splitn(3, ..) yields at most 2 remaining segments"),
    };
    Some(Directive { path: PathBuf::from(path), start, end })
}

fn parse_line_number(raw: &str) -> Option<usize> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        trimmed.parse().ok()
    }
}

/// 1-indexed, inclusive on both ends; an absent bound extends to the start/end of the file.
fn slice_lines(contents: &str, start: Option<usize>, end: Option<usize>) -> String {
    if start.is_none() && end.is_none() {
        return contents.to_string();
    }
    let lines: Vec<&str> = contents.lines().collect();
    let start_idx = start.map(|n| n.saturating_sub(1)).unwrap_or(0).min(lines.len());
    let end_idx = end.unwrap_or(lines.len()).min(lines.len());
    if start_idx >= end_idx {
        return String::new();
    }
    let mut result = lines[start_idx..end_idx].join("\n");
    result.push('\n');
    result
}
