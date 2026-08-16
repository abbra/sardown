# Style examples

Reference `style.toml` files demonstrating regional paper-size conventions, built on the
`[page]` section of md2pdf's stylesheet schema (see
`docs/superpowers/specs/2026-08-16-stylesheet-configuration-design.md`).

| File | Format | Margin | Convention |
|---|---|---|---|
| `us-letter.toml` | Letter | 25.4mm (1in) | US business/academic standard |
| `us-legal.toml` | Legal | 25.4mm (1in) | US legal documents |
| `eu-a4.toml` | A4 | 20mm | EU/international business and technical documents |
| `eu-a3.toml` | A3 | 20mm | Larger-format documents (diagrams, posters) |
| `eu-a5.toml` | A5 | 15mm | Booklet-style documents |

These are parsed and validated against `md2pdf-style`'s `Stylesheet::load` in
`crates/md2pdf-style/tests/style_examples.rs`. Loading a stylesheet is fully implemented today;
passing one to `md2pdf render`/`render-book` via a `--style` flag is a later phase of the
stylesheet configuration feature and isn't wired up yet.
