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

## Documentation

The full guide (installation, Markdown/diagram support, book and slide
rendering, the stylesheet reference, and troubleshooting) is in
[`docs/`](./docs/src/introduction.md), built with mdBook.

## License

GPL-3.0-or-later — see [LICENSE](./LICENSE).
