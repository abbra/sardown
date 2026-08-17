# Introduction

md2pdf is a native Rust engine that renders Markdown — either a single file
or a full mdBook source tree — directly to PDF, without going through an
HTML/browser rendering step. It parses Markdown, lays out pages itself, and
emits PDF/A-2b compliant output.

## Why md2pdf

- **No browser dependency.** Rendering is done directly against the PDF
  object model (via [krilla](https://github.com/typst/krilla)), so there's
  no headless Chrome, no wkhtmltopdf, no external runtime to install.
- **Real pagination.** Content is laid out page by page, with page breaks,
  widow/orphan control on headings, tables that don't split mid-row, and
  code blocks/blockquotes that carry their background or border segments
  across a page break correctly.
- **Whole-book rendering.** Point `render-book` at an mdBook-style source
  tree (`book.toml` + `src/SUMMARY.md`) and get one combined PDF with
  working cross-chapter links, not just per-chapter files.
- **Configurable styling.** Page geometry, typography, headings, tables,
  code blocks, and running headers/footers are all controlled by an
  external TOML stylesheet — no stylesheet at all reproduces sensible
  built-in defaults.
- **Mermaid diagrams.** Fenced ` ```mermaid ` code blocks are rendered as
  real vector diagrams, not screenshots.

## What it isn't

md2pdf is not a general-purpose HTML-to-PDF converter and doesn't execute
JavaScript or apply CSS. It supports the Markdown feature set described in
[Writing Markdown](./markdown-support.md) plus Mermaid diagrams — anything
outside that (raw embedded HTML, custom CSS, browser-rendered widgets) is
out of scope.

## How this book is organized

- [Installation](./installation.md) and [Quick Start](./quick-start.md) get
  you rendering your first PDF.
- [Writing Markdown](./markdown-support.md) and [Diagrams](./diagrams.md)
  cover the supported document syntax.
- [Rendering Books](./books/index.md) covers multi-chapter documents.
- [Styling Your Documents](./styling/index.md) is the guide to the
  stylesheet system — how to change fonts, colors, page size, code themes,
  and running headers/footers.
- [Command-Line Reference](./cli-reference.md) and
  [Stylesheet Reference](./stylesheet-reference.md) are the complete,
  field-by-field references.
- [Troubleshooting](./troubleshooting.md) covers common warnings and what
  they mean.
