# Page Setup

## Format presets

```toml
[page]
format = "letter"   # default
margin_mm = 25.4
```

`format` accepts a named preset: `letter` (215.9×279.4mm, the default),
`legal` (215.9×355.6mm), `a4` (210×297mm), `a3` (297×420mm), or `a5`
(148×210mm). `margin_mm` is the same margin applied to all four sides,
defaulting to 25.4mm (1 inch).

## Custom dimensions

To use a page size outside the named presets, set both `width_mm` and
`height_mm` explicitly — this overrides `format` entirely:

```toml
[page]
width_mm = 176.0
height_mm = 250.0
margin_mm = 20.0
```

Setting only one of `width_mm`/`height_mm` (not both) is a validation
error at load time — `md2pdf` will refuse to run with a stylesheet like
that rather than silently guessing the other dimension.

## Page numbering format

```toml
[page.numbering]
format = "arabic"   # default; also "roman_lower", "roman_upper"
start_at = 1
```

This controls *how a page number is formatted* wherever `{page}`/
`{total_pages}` appear in a header or footer template (see
[Headers and Footers](./headers-and-footers.md)) — it has no visible
effect unless a header or footer is also enabled. `start_at` lets the
first page be numbered something other than 1 (e.g. for a document meant
to be bound after a separately-numbered preface).
