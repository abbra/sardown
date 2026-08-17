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
error at load time — `sardown` will refuse to run with a stylesheet like
that rather than silently guessing the other dimension.

## Asymmetric (two-sided binding) margins

```toml
[page]
inner_margin_mm = 30.0
outer_margin_mm = 20.0
```

For a document meant to be printed and bound, the margin nearest the
spine ("inner") is usually larger than the outer margin, so text isn't
crowded into the binding — and which physical side is "inner" alternates
every page. Setting both `inner_margin_mm` and `outer_margin_mm` enables
this: odd physical pages (recto) get `inner_margin_mm` on the left and
`outer_margin_mm` on the right; even physical pages (verso) get the
reverse. `margin_mm` continues to apply to the top and bottom on every
page either way. Setting only one of the two (not both) is a validation
error, same as `width_mm`/`height_mm`. This is independent of
`[header]`/`[footer]`'s own `mode = "two_sided"` — you can use asymmetric
margins with no header/footer at all, or a two-sided header/footer with
symmetric margins.

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

## Restarting numbering partway through

```toml
[page.numbering]
format = "roman_lower"   # front matter: i, ii, iii, ...

[[page.numbering.resets]]
at_heading = "chapter-one"
format = "arabic"        # body restarts at 1, 2, 3, ...
start_at = 1
```

A common convention for printed books: front matter (preface, table of
contents) is numbered with lowercase roman numerals, then the main body
restarts at arabic `1`. `[[page.numbering.resets]]` (an array — you can
add more than one) restarts the displayed page count, and optionally the
format, from a named heading's page onward. `at_heading` is the heading's
id — the same slug used for cross-reference links (a heading's id is
whatever `#heading-text` would resolve to; see
[Cross-References](../books/cross-references.md) for how ids are derived).
`format`/`start_at` each default to `"arabic"`/`1` if omitted, so
`at_heading` is the only field you must set.

A reset naming a heading id that doesn't exist in the document is ignored
with a warning at render time — there's no way to validate heading ids
before the document is parsed. `{total_pages}` is unaffected by resets: it
always shows the document's literal physical page count, not a
per-segment count.
