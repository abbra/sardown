# Style examples

Reference `style.toml` files for md2pdf's stylesheet configuration (see
`docs/superpowers/specs/2026-08-16-stylesheet-configuration-design.md`). Pass one with
`md2pdf render --style <path>` / `md2pdf render-book --style <path>`, or drop a copy named
`style.toml` at a book's root for `render-book` to pick up automatically.

## Regional traditions

Page geometry plus the typographic conventions common to each region: body/heading typeface,
body size, heading accent color, and the region's own page-numbering convention (a plain
centered page number for the US tradition; a "page / total" footer for the EU tradition).

| File | Format | Margin | Convention |
|---|---|---|---|
| `us-letter.toml` | Letter | 25.4mm (1in) | US business/academic standard |
| `us-legal.toml` | Legal | 25.4mm (1in) | US legal documents ("Page X of Y" footer) |
| `eu-a4.toml` | A4 | 20mm | EU/international business and technical documents |
| `eu-a3.toml` | A3 | 20mm | Larger-format documents (diagrams, posters) |
| `eu-a5.toml` | A5 | 15mm | Booklet-style documents |

## Classic document styles

Presets modeled on well-known document-type conventions, independent of region.

| File | Style | Notable choices |
|---|---|---|
| `university-paper.toml` | Academic paper (APA/MLA-inspired) | Serif body, top-right running page number, 0.5in block-quote indent |
| `technical-manual.toml` | Printed reference manual | Tighter margins, dark-blue headings, `header_bar` code labels on a light theme, chapter-name + page-number footer |
| `technical-guide.toml` | Docs-site/tutorial guide | Teal accent, generous spacing, `inline` code labels on a dark theme (with a matching dark code-block background -- see the file's own comment on why that pairing matters) |
| `fiction.toml` | Novel manuscript | A5 paperback trim, serif body, large chapter-opening space-before, page number suppressed on chapter-opening pages |

## Notes

- These are parsed and validated against `md2pdf-style`'s `Stylesheet::load` in
  `crates/md2pdf-style/tests/style_examples.rs`.
- `font_family` values throughout these files (e.g. "Times New Roman", "Helvetica",
  "Garamond") are recorded as each style's intended typeface, but aren't rendered yet --
  font selection isn't wired into layout in this phase (see the design spec's "Font
  Resolution" section). Every other field takes effect today.
