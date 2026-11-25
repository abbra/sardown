# Writing Markdown

sardown parses CommonMark plus GitHub-style tables, via
[pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark). This
page lists exactly what's supported and, just as importantly, what isn't.

## Headings

`#` through `######` (levels 1–6), each with its own default size (falling
from 28pt at H1 down to 12pt at H6) and, for level-1 headings inside a
book, used to identify chapter boundaries. Sizes, color, and font are all
configurable per level — see [Headings](./styling/headings.md).

## Paragraphs and inline styling

Regular paragraphs, **bold** (`**text**`), *italic* (`*text*`), and
combinations of the two, plus links (`[text](url)`). Soft and hard line
breaks within a paragraph both collapse to a single space (a hard break
does not currently force a visual line break).

Inline code spans (`` `code` ``) render in a monospace font, distinct
from the surrounding body text. They don't yet get a background
highlight box the way fenced code blocks do — that's a separate,
not-yet-implemented rendering feature (drawing a background behind an
individual run within a paragraph, rather than behind a whole block).

`~~strikethrough~~` renders with a line drawn through the text, in the
text's own color. Footnotes aren't specially handled; they render as
their literal source text.

Task list items (`- [ ]`/`- [x]`) render a checkbox glyph (☐/☑) in place
of the literal brackets:

```markdown
- [ ] Not done yet
- [x] Done
```

## Lists

Both ordered (`1.`) and unordered (`-`/`*`) lists, tight or loose, nested
to any depth. An unordered item gets a "•" marker; an ordered item gets
its own number, honoring a non-default start (`5. Fifth item` starts
numbering at 5, not 1). A marker is only added when the item's own first
line of content is a plain paragraph — the overwhelming common case; an
item starting directly with a nested list, image, or code block renders
without one.

## Tables

GitHub-style pipe tables, including column alignment markers (`:---`,
`:---:`, `---:`). Column widths are computed automatically from content;
see [Tables](./styling/tables.md) for styling (padding, text size, row
height).

## Blockquotes

`>` blockquotes, including nested block content (paragraphs, lists, code
blocks) inside them, rendered with a left border. See
[Structural Elements](./styling/structural-elements.md).

## Thematic breaks

`---` (on its own line, not as a heading underline) renders as a
horizontal rule.

## Code blocks

Fenced code blocks (` ``` `), with or without a language tag, are syntax
highlighted (see [Code Blocks](./styling/code-blocks.md) for themes and
per-language styling). A ` ```mermaid ` fence is treated specially — see
[Diagrams](./diagrams.md) — rather than highlighted as code.

## Images

`![alt](path)` embeds a local PNG, JPEG, or SVG file, resolved relative to
the Markdown file's own directory (or, inside a book, relative to that
chapter's own directory). An SVG renders as vector content (the same
mechanism a Mermaid diagram uses), scaled to fit the page width — and,
like a Mermaid diagram, scaled down further if it would still be taller
than a full page. **External image URLs (`http://`/`https://`) are
not fetched** — they're skipped with a warning on stderr rather than
silently failing or blocking on a network request.

`![alt](data:image/png;base64,...)` embeds the image data directly —
`image/png`, `image/jpeg`, and `image/svg+xml` are all supported, decoded
the same way as a referenced file (an embedded SVG still renders as vector
content). Only the base64-encoded form of a data URI is supported; a
percent-encoded one is skipped with a warning.

## Math

A paragraph consisting only of `$$ ... $$` is typeset as a display equation
(LaTeX syntax, rendered as vector outlines at the body text size and centered
on the text column). A LaTeX error is reported on stderr and the equation is
skipped. Inline `$...$` math is **not** typeset: it renders as italic source
text, because a text line can't carry an inline graphic.

## Footnotes

`text[^name]` with a `[^name]: note text` definition anywhere in the file.
Notes are numbered in order of citation, the marker is a superscript digit,
and the note is set in smaller type at the bottom of the page holding the
reference, under a short rule (see `[footnote]` in the
[stylesheet reference](./stylesheet-reference.md)). A definition nobody
cites is dropped. A reference inside a `::columns` block is not numbered
or placed.

## Front matter

A `---` YAML block at the very top of the file sets a title block:

```yaml
---
title: A Study of Things
author: [Jane Doe, Example University]   # one centered line per entry
date: November 2026
abstract: |
  Markdown text, set as an indented block under an "Abstract" label.
---
```

All keys are optional (`references` is described under Citations below). The title uses the level-1 heading style; `author`
may be a single string or a list. Invalid YAML is ignored with a warning.

## Citations

List entries under `references:` in the front matter (key → one Markdown
paragraph), then cite with `[@key]` or `[@a; @b]`:

```yaml
references:
  knuth84: "D. Knuth. *The TeXbook*. Addison-Wesley, 1984."
```

Citations are numbered in order of first use and shown as `[1]` / `[1, 2]`.
A "References" section (unnumbered) with the cited entries is appended at the
end; entries nobody cites are dropped. An unknown key is left as written, with
a warning on stderr. Each citation number links to its entry in the References.

## Captions, numbering and cross-references

- A paragraph starting `Figure: ` or `Table: ` becomes a caption with a bold,
  auto-numbered "Figure 1." / "Table 1." label (each counted separately).
  A caption stays on the same page as the image, table or equation directly
  after it, or else directly before it.
- Headings are numbered when `[heading] number_from_level` is set (the arXiv
  preset sets it to `2`: `##` is "1", `###` is "1.1").
- A `$$` equation containing `\label{eq:x}` is numbered "(1)" at its right.
- Give a heading `{#sec:intro}`, or end a caption with `{#fig:one}`, to name
  it. Writing `@sec:intro`, `@fig:one` or `@eq:x` in text expands to
  "Section 1", "Figure 1" or "Eq. (1)", linked to the target. An
  `@name` that matches no label (an e-mail address, a handle) is left alone.

## What's out of scope

sardown is not an HTML renderer: raw embedded HTML blocks/inline HTML,
custom CSS, and JavaScript have no effect. There's also no text-wrapping
control beyond the stylesheet's own alignment setting (see
[Typography](./styling/typography.md)) — no manual hyphenation, no
per-paragraph overrides of a document-wide style choice.
