# Styling Your Documents

Every visual aspect of a rendered document — page size and margins, fonts,
heading sizes and colors, table padding, code block themes, running
headers/footers — is controlled by a **stylesheet**: a TOML file you write
and pass to md2pdf. With no stylesheet at all, md2pdf uses built-in
defaults that reproduce a plain, sensible-looking document (US Letter,
sans-serif body text, black headings, no header/footer).

## Applying a stylesheet

```bash
md2pdf render document.md -o output.pdf --style my-style.toml
md2pdf render-book my-book -o output.pdf --style my-style.toml
```

`render-book` has one more option: if `--style` isn't given, it
automatically looks for a `style.toml` file at the book's own root
(`<book_root>/style.toml`) and uses it if present. Precedence is always:
**explicit `--style` flag** > **auto-discovered `<book_root>/style.toml`**
> **built-in defaults**.

## Partial overrides

A stylesheet only needs to set the fields it wants to change — everything
else falls back to its default. For example, this is a complete, valid
stylesheet that only changes the page format and leaves every other
setting (fonts, colors, table padding, and so on) at its default:

```toml
[page]
format = "a4"
```

## Sections

A stylesheet is organized into sections matching the parts of a document
they control:

| Section | Controls |
|---|---|
| `[document]` | Title/author, available to header/footer templates as `{title}`/`{author}` — see [Headers and Footers](./headers-and-footers.md#zones-and-templates) |
| `[page]` | [Page format, margins, page numbering](./page.md) |
| `[typography]` | [Body font, size, color, alignment](./typography.md) |
| `[heading]` | [Heading sizes, color, font, per-level overrides](./headings.md) |
| `[blockquote]`, `[thematic_break]`, `[list]` | [Structural element styling](./structural-elements.md) |
| `[table]` | [Table padding and text size](./tables.md) |
| `[code_block]` | [Syntax theme, labels, per-language overrides](./code-blocks.md) |
| `[header]`, `[footer]` | [Running headers/footers, page numbering, templates](./headers-and-footers.md) |
| `[toc]` | [Table of contents and PDF outline generation](./table-of-contents.md) |

For a complete, field-by-field listing of every value each section
accepts, see the [Stylesheet Reference](../stylesheet-reference.md). For
ready-made stylesheets you can use directly or adapt, see the
[Style Presets Gallery](./presets.md).

## A note on fonts

Setting `font_family` anywhere in a stylesheet (typography, headings, code
blocks, headers/footers) only works if that font is actually available —
either already installed as a system font, or loaded from a directory via
`typography.font_dirs`. An unresolvable font name doesn't fail the
render; it falls back to a generic sans-serif font with a warning on
stderr. See
[Font Resolution](../troubleshooting.md#unknown-font-family-warnings).
