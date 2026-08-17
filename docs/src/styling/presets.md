# Style Presets Gallery

`docs/style-examples/` in the repository ships nine ready-to-use
stylesheets: five covering regional typographic traditions, and four
covering classic document-type conventions. Use one directly with
`--style`, or copy and adapt it as a starting point.

```bash
sardown render document.md -o output.pdf --style docs/style-examples/eu-a4.toml
```

## Regional traditions

| File | Format | Margin | Convention |
|---|---|---|---|
| `us-letter.toml` | Letter | 25.4mm (1in) | US business/academic standard |
| `us-legal.toml` | Legal | 25.4mm (1in) | US legal documents ("Page X of Y" footer) |
| `eu-a4.toml` | A4 | 20mm | EU/international business and technical documents |
| `eu-a3.toml` | A3 | 20mm | Larger-format documents (diagrams, posters) |
| `eu-a5.toml` | A5 | 15mm | Booklet-style documents |

The US presets use a serif body face and a plain, centered footer page
number. The EU presets use a sans-serif body face and a "page / total"
footer in the bottom-right, a common EU/DIN convention. All five show the
footer on every page, including the first — appropriate for a standalone
document (a letter, a report) rather than a multi-chapter book.

## Classic document styles

### University Paper

APA/MLA-inspired: Letter page, 1-inch margins, a serif body face at the
traditional 12pt, and a simple running page number in the top-right
header (no footer) — shown on every page including the first. Block
quotes get the APA/MLA 0.5-inch indent rather than the default.

### Technical Manual

The style of a printed reference manual: tighter 20mm margins, a
sans-serif body at 10.5pt, dark-blue headings, and code blocks with a
labeled header bar on a light "Solarized (light)" theme. The footer names
the current chapter (left) and page number (right) — the classic
printed-manual running-footer layout — and, matching real printed manuals,
is suppressed on each chapter's own opening page.

### Technical Guide

A friendlier tutorial/docs-site style: generous margins and heading
spacing, a teal accent color, and code blocks with an inline label on a
dark `base16-ocean.dark` theme — paired with a matching dark code-block
background (`#2b303b`), demonstrating the pairing rule described in
[Code Blocks](./code-blocks.md#pairing-a-dark-theme-with-a-matching-background).

### Fiction

A novel/manuscript style: A5 trim (approximating a common paperback
size), a serif body face, and a large chapter-opening space-before factor
that pushes each chapter heading partway down the page — the conventional
look of a printed novel. A plain centered footer page number is
suppressed on each chapter's own opening page, matching how printed
novels typically leave the page number off a chapter's first page.

## A note on fonts

Every preset that names a specific font (e.g. `"Times New Roman"`,
`"Garamond"`, `"Helvetica"`) only gets that look if the font is actually
available on the machine rendering the document — otherwise it falls back
to a generic sans-serif font with a warning. See
[Font Resolution](../troubleshooting.md#unknown-font-family-warnings).
