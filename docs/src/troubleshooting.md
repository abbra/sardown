# Troubleshooting

md2pdf's general philosophy is to keep rendering and produce output even
when something can't be handled perfectly, printing a warning on stderr
and degrading gracefully rather than aborting the whole document. This
page explains the warnings you're most likely to see.

## Unknown font family warnings

```
warning: unknown font family "Times New Roman"; falling back to a sans-serif font
```

A `font_family` value (in `[typography]`, `[heading]`, `[code_block]`, or
`[header]`/`[footer]`) that isn't one of the five generic keywords
(`serif`, `sans-serif`, `monospace`, `cursive`, `fantasy`) is treated as a
literal font name and checked against every font md2pdf can actually load.
If it isn't found — not installed as a system font, and not present in any
directory listed in `typography.font_dirs` — the render still succeeds,
using a generic sans-serif font instead of aborting.

**Fix:** install the named font system-wide, or point `font_dirs` at a
directory containing it:

```toml
[typography]
font_family = "Custom Serif"
font_dirs = ["./fonts"]
```

## Unknown syntax theme warnings

```
warning: unknown syntax theme "monokai"; falling back to InspiredGitHub
```

`[code_block].syntax_theme` only recognizes syntect's own bundled theme
names (listed in [Code Blocks](./styling/code-blocks.md#syntax-theme)). An
unrecognized name falls back to `InspiredGitHub` rather than failing.

## "character ... is not supported by any available font"

```
warning: character 'é' is not supported by any available font; dropping it from the output
```

No loaded font has a glyph for that character. It's dropped from the
output rather than rendered as a "tofu box" (`.notdef` glyph) — PDF/A
(which md2pdf always produces) forbids emitting that glyph, and a strict
PDF/A validator would reject the whole document if it appeared even once.

**Fix:** load a font (via `font_dirs` or a system font) with actual
coverage for the character set you need — this comes up most often with
non-Latin scripts and symbols when the resolved font is a narrow-coverage
Latin-only face.

## "skipping external image (not fetched)"

```
warning: skipping external image (not fetched): https://example.com/photo.png
```

External (`http://`/`https://`) image URLs are never fetched — see
[Writing Markdown](./markdown-support.md#images). Download the image and
reference it as a local, relative path instead.

## Diagram compile warnings

```
warning: failed to render Mermaid diagram at notes.md:7:11: ...
```

See [Diagrams](./diagrams.md#when-a-diagram-fails-to-compile) — the
diagram is left out of the output, but the rest of the document still
renders.

## Failed to read chapter warnings (books only)

```
warning: failed to read chapter src/missing.md: No such file or directory (os error 2)
```

A chapter listed in `SUMMARY.md` couldn't be read (usually a typo in the
path). That one chapter is skipped; the rest of the book still renders —
this is a warning, not a fatal error, specifically so one broken chapter
reference doesn't block rendering everything else.

## Invalid stylesheet errors

Unlike the warnings above, these stop the render entirely — the input
isn't something md2pdf can proceed with:

- Invalid TOML syntax.
- `[page]` sets only one of `width_mm`/`height_mm` (both or neither is
  required).
- A header/footer template contains an unknown `{placeholder}` or an
  unterminated `{`.

The error message names the specific problem (and, for a bad placeholder,
the exact bad token) rather than a generic parse failure.
