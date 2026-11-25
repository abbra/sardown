# sardown

A native Rust engine that renders Markdown — a single file, a full mdBook
source tree, or a slide deck — directly to PDF/A-2b, with no headless
browser or external runtime involved.

## Highlights

- **No browser dependency.** Rendering goes straight against the PDF object
  model (via [krilla](https://github.com/typst/krilla)) — no headless
  Chrome, no wkhtmltopdf.
- **Real pagination.** Page breaks, widow/orphan control on headings,
  tables that don't split mid-row, and code blocks/blockquotes whose
  background or border carries correctly across a page break.
- **Whole-book rendering.** `render-book` combines an mdBook-style source
  tree (`book.toml` + `src/SUMMARY.md`) into one PDF with working
  cross-chapter links.
- **Native slide decks.** `render-slides` turns a `---`-separated Markdown
  deck into a slide-per-page PDF, with named layouts, side-by-side
  `::columns`, auto-shrink-to-fit, and per-layout background images.
- **Configurable styling.** Page geometry, typography, headings, tables,
  code blocks, and running headers/footers are all controlled by an
  external TOML stylesheet.
- **Mermaid diagrams.** Fenced ` ```mermaid ` blocks render as real vector
  diagrams, not screenshots.

## Building

sardown is a Cargo workspace; there's no published crate or prebuilt binary
yet.

```bash
git clone https://github.com/abbra/sardown
cd sardown
cargo build --release
```

The binary is produced at `target/release/sardown`, or install it onto your
`PATH` with `cargo install --path crates/sardown-cli`.

## Quick start

```bash
sardown render hello.md -o hello.pdf
sardown render-book path/to/my-book -o my-book.pdf
sardown render-slides slides.md -o slides.pdf
```

## arXiv-style papers

Two presets in [`docs/style-examples/`](./docs/style-examples/) turn Markdown into a
preprint that looks like a LaTeX-typeset arXiv paper:

| Preset | Layout |
|---|---|
| `arxiv.toml` | One column: Letter, 10pt Times-style justified body, numbered sections |
| `arxiv-twocolumn.toml` | Two columns with 0.75in margins; title block and abstract stay full width |

```bash
sardown render paper.md --style docs/style-examples/arxiv-twocolumn.toml -o paper.pdf
```

The presets use Nimbus Roman (a metric-compatible Times clone); without it sardown falls
back to a sans-serif font with a warning. `paper.md` can use:

````markdown
---
title: A Study of Things
author: [Jane Doe, Example University]
abstract: |
  We study things.
references:
  knuth84: "D. Knuth. *The TeXbook*. Addison-Wesley, 1984."
---

## Introduction {#sec:intro}

Following [@knuth84], @eq:e holds.[^1] See @sec:intro and @fig:f.

$$ E = mc^2 \label{eq:e} $$

![A plot](plot.png)

Figure: A plot of things. {#fig:f}

[^1]: A footnote, set at the bottom of the page.
````

Numbered sections, equations, figures and tables; display math; footnotes; numbered
citations with a References list; and linked cross-references all work in both presets. In
the two-column preset, images, tables and display equations span both columns, and the
last page's columns are balanced. Inline `$...$` math is not typeset. Details are in
[Markdown support](./docs/src/markdown-support.md).

## Documentation

The full guide (installation, Markdown/diagram support, book and slide
rendering, the stylesheet reference, and troubleshooting) is in
[`docs/`](./docs/src/introduction.md), built with mdBook.

## License

GPL-3.0-or-later — see [LICENSE](./LICENSE).
