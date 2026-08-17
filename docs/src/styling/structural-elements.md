# Structural Elements

## Blockquotes

```toml
[blockquote]
border_color = "#b4b4b4"   # default
border_width_pt = 2.0
indent_pt = 18.0
```

Controls the vertical border drawn to the left of blockquote content and
how far the quoted content itself is indented from the border. A larger
`indent_pt` (e.g. `36.0`, half an inch) is a common choice for academic
styles that follow APA/MLA block-quote conventions.

## Thematic breaks

```toml
[thematic_break]
color = "#c8c8c8"   # default
width_pt = 1.0
```

Controls the horizontal rule drawn for a `---` thematic break.

## Lists

```toml
[list]
indent_pt = 18.0   # default
```

Controls how far list item content is indented from the surrounding
content's left edge. Applies uniformly at every nesting depth (a
second-level nested list is indented by `indent_pt` again, relative to its
parent item).
