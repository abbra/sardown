# SUMMARY.md Format

`SUMMARY.md` is read as plain Markdown, and its list nesting/heading
structure is interpreted as the book's chapter structure — the same basic
model real mdBook uses, so there's no separate custom format to learn for
the common case, including [prefix chapters](#prefix-chapters).

## Basic structure

```markdown
# Summary

- [Introduction](introduction.md)
- [Getting Started](getting-started.md)
  - [Installation](getting-started/installation.md)
  - [Quick Start](getting-started/quick-start.md)
- [Reference](reference.md)
```

- The **first heading** (`# Summary` above) is the file's own title — it's
  consumed and not treated as part of the chapter structure.
- Each top-level list item is a chapter. `[Title](path)` gives the
  chapter's title and its file, resolved relative to the source directory
  (`src/` by default).
- Indented (nested) list items become that chapter's sub-chapters,
  recursively, to any depth.

## Draft chapters

A list item with no link — just `[Title]()` or plain text — is a draft
chapter: it contributes no content of its own, but its nested children (if
any) are still walked and included.

## Part titles and separators

```markdown
# Summary

- [Chapter One](one.md)

# Part Two

- [Chapter Two](two.md)

---

- [Appendix](appendix.md)
```

A heading *after* the first one starts a new "part" (mdBook shows these as
sidebar group labels in its HTML output); a `---` thematic break is a
separator. Both are recognized during parsing today, but neither currently
gets a visual treatment in the combined PDF body — chapters before and
after one still concatenate normally.

## Prefix chapters

Real mdBook lets a chapter appear *before* the first `-` list item, as a
bare link with no list marker — commonly used for an introduction or
preface that shouldn't be numbered like the rest of the chapters:

```markdown
# Summary

[Introduction](introduction.md)

- [Chapter One](chapter-1.md)
```

md2pdf recognizes this the same way: `[Introduction](introduction.md)`
above is included as its own chapter, positioned before `Chapter One`,
exactly as written. This book's own `SUMMARY.md` uses exactly this
pattern for its own introduction. One difference from a regular list-item
chapter: a prefix chapter can't have nested sub-chapters (there's no list
for them to nest under) — if you need nested children, use a regular
first list item instead.

## Chapters with no heading of their own

If a chapter file's content doesn't start with a top-level (`#`) heading,
md2pdf synthesizes one from the title given in `SUMMARY.md`, so the chapter
still has a visible heading at the top of its page. If the chapter *does*
start with its own `#` heading, that's used as-is and nothing is
synthesized.
