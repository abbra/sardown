# Diagrams

sardown renders [Mermaid](https://mermaid.js.org) diagrams as real vector
graphics — not screenshots, and without launching a browser — using the
pure-Rust [merman](https://github.com/zed-industries/merman) renderer.

## Writing a diagram

Use a ` ```mermaid ` fenced code block anywhere a normal code block would
go:

````markdown
```mermaid
flowchart TD
    A[Parse] --> B[Layout]
    B --> C[Emit PDF]
```
````

sardown compiles this to an SVG internally and places it as a page element,
sized to fit within the content width.

## Supported diagram types

Diagram type support depends entirely on merman's own coverage. Flowcharts
and sequence diagrams are exercised by sardown's own test suite and known to
work; consult merman's documentation for its full supported diagram set.

## When a diagram fails to compile

If a diagram's Mermaid source has a syntax error, sardown doesn't fail the
whole render — it prints a warning naming the file and the *exact* line
and column inside the diagram's own source (not just the location of the
opening ` ``` ` fence) and continues, leaving that one diagram out of the
output. For example:

```
warning: failed to render Mermaid diagram at notes.md:7:11: ...
```

points at the actual offending token on line 7 of `notes.md`, even though
the fence itself might open several lines earlier.

## Diagrams inside books

When a diagram lives inside a chapter of a book rendered with
`render-book`, warnings name the chapter's own path (as listed in
`SUMMARY.md`), not an internal synthetic ID — so you can find the file to
fix directly from the warning.

## The `ondine` renderer

By default diagrams are drawn by merman, which reproduces Mermaid's own
look. A build with the `ondine` feature can draw them with
[Ondine's compiler](https://github.com/wevm/ondine) instead: plain,
monochrome diagrams set in a monospace font, colored from your stylesheet
rather than from Mermaid themes (`classDef`, `style` and `%%{init}%%` colors
are ignored).

```sh
cargo install --path crates/sardown-cli --features ondine
```

```toml
[mermaid]
renderer = "ondine"
border_color = "#609538"
node_fill = "#e3f2d7"
line_color = "#00689e"
```

- Text uses `[code_block.default].font_family` unless `[mermaid]` sets
  `font_family`; Ondine measures text as monospace, so a font whose glyphs
  advance 0.6 em (Red Hat Mono, DejaVu Sans Mono) fits its boxes exactly.
- A diagram type Ondine does not support is drawn by merman, with a note.
- Ondine is not on crates.io, so the feature builds it from a pinned git
  revision (network access needed to fetch it, and a crate built with the
  feature cannot be published to crates.io).
- Without the feature, `renderer = "ondine"` prints a warning and uses merman.

## Text rendering inside diagrams

Diagram labels need a resolvable font to shape their text (the same
font-resolution rules as everywhere else — see
[Font Resolution](./troubleshooting.md#unknown-font-family-warnings)
apply). If no usable font is available at all, a diagram's boxes and
arrows (pure geometry) still render, but its text labels may be silently
dropped instead of appearing as visible text.
