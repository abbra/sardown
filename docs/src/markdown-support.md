# Writing Markdown

md2pdf parses CommonMark plus GitHub-style tables, via
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
to any depth.

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

## What's out of scope

md2pdf is not an HTML renderer: raw embedded HTML blocks/inline HTML,
custom CSS, and JavaScript have no effect. There's also no text-wrapping
control beyond the stylesheet's own alignment setting (see
[Typography](./styling/typography.md)) — no manual hyphenation, no
per-paragraph overrides of a document-wide style choice.
