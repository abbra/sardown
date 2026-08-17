# Code Blocks

```toml
[code_block]
syntax_theme = "InspiredGitHub"   # default
label_style = "none"
default_label = "text"

[code_block.default]
background = "#f5f5f5"
font_family = "monospace"
font_size_pt = 10.0
label_color = "#666666"
label_background = "#e0e0e0"
```

## Syntax theme

`syntax_theme` selects one of [syntect](https://github.com/trishume/syntect)'s
bundled themes by name:

- `InspiredGitHub` (default, light)
- `Solarized (light)`
- `Solarized (dark)`
- `base16-eighties.dark`
- `base16-mocha.dark`
- `base16-ocean.dark`
- `base16-ocean.light`

An unrecognized theme name falls back to `InspiredGitHub` with a warning
on stderr, rather than failing the render.

### Pairing a dark theme with a matching background

This renderer only takes **foreground** text colors from the syntect
theme — the code block's own background rectangle always comes from the
separate `[code_block.default].background` (or a per-language override)
value, set independently. A dark theme's foreground colors are calibrated
for a dark backdrop; pairing one with the default *light* background makes
the highlighted text hard to read or nearly invisible. If you use a dark
theme, set a matching dark `background` too — see the
[`technical-guide.toml`](./presets.md#technical-guide) preset for a worked
example (`base16-ocean.dark` paired with `#2b303b`).

## Labels

`label_style` controls whether — and how — a code block shows a label
naming its language:

- `"none"` (default) — no label.
- `"corner"` — a small badge in the top-right corner of the block, drawn
  over an enlarged top padding reserved for it.
- `"header_bar"` — a full-width bar above the code, showing the label.
- `"inline"` — the label is prepended as the first line of the code
  block's own text.

The label text itself is the fence's language tag, title-cased (` ```rust `
becomes "Rust"). Fences with no language tag use `default_label` instead
(default `"text"`).

## Per-language overrides

```toml
[code_block.languages.rust]
label = "Rust Example"
background = "#fdf0e6"

[code_block.languages.python]
font_size_pt = 9.0
```

Each language key (matching the fence's own tag, e.g. `rust` for
` ```rust `) can override `label`, `background`, `font_family`,
`font_size_pt`, `label_color`, and `label_background` independently.
Anything not overridden for a given language falls back to
`[code_block.default]`.

## What's not affected by this section

Inline code spans (`` `text` ``) don't use any of this — they render as
plain body text (see
[Writing Markdown](../markdown-support.md#paragraphs-and-inline-styling)).
