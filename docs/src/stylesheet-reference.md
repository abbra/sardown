# Stylesheet Reference

Complete, field-by-field listing of every stylesheet section. See
[Styling Your Documents](./styling/index.md) for the prose guide covering
*why* and *how* to use each of these; this page is the enumerated
reference.

Every field is optional — an absent field always falls back to the default
value shown here. A color field accepts either a 6-digit hex string
(`"#rrggbb"`, with or without the `#`) or an `[r, g, b]` byte array.

## `[document]`

| Field | Type | Default |
|---|---|---|
| `title` | string | `""` |
| `author` | string | `""` |
| `date` | string | `""` (auto-filled with today's date at render time if still empty) |

Available to `[header]`/`[footer]` templates as `{title}`/`{author}`/
`{date}`. The `--title`/`--author`/`--date` CLI flags override these if
both are given.

## `[page]`

| Field | Type | Default |
|---|---|---|
| `format` | `"letter"` \| `"legal"` \| `"a4"` \| `"a3"` \| `"a5"` | `"letter"` |
| `width_mm` | number | unset |
| `height_mm` | number | unset |
| `margin_mm` | number | `25.4` |
| `inner_margin_mm` | number | unset |
| `outer_margin_mm` | number | unset |

Setting `width_mm`/`height_mm` overrides `format` entirely; both must be
set together, or loading the stylesheet is an error. Setting
`inner_margin_mm`/`outer_margin_mm` enables asymmetric two-sided-binding
margins (both must be set together too) — see
[Page Setup](./styling/page.md#asymmetric-two-sided-binding-margins).

### `[page.numbering]`

| Field | Type | Default |
|---|---|---|
| `format` | `"arabic"` \| `"roman_lower"` \| `"roman_upper"` | `"arabic"` |
| `start_at` | integer | `1` |
| `resets` | array of `[[page.numbering.resets]]` tables | `[]` |

#### `[[page.numbering.resets]]`

| Field | Type | Default |
|---|---|---|
| `at_heading` | string (a heading id) | required, no default |
| `format` | `"arabic"` \| `"roman_lower"` \| `"roman_upper"` | `"arabic"` |
| `start_at` | integer | `1` |

Restarts numbering (with its own `format`/`start_at`) from `at_heading`'s
page onward — see
[Restarting numbering partway through](./styling/page.md#restarting-numbering-partway-through).

## `[typography]`

| Field | Type | Default |
|---|---|---|
| `font_family` | string | `"sans-serif"` |
| `font_dirs` | list of paths | `[]` |
| `use_system_fonts` | boolean | `true` |
| `body_size_pt` | number | `12.0` |
| `body_color` | color | `"#000000"` |
| `alignment` | `"left"` \| `"right"` \| `"center"` \| `"justify"` | `"left"` |

## `[heading]`

| Field | Type | Default |
|---|---|---|
| `space_before_factor` | number | `0.8` |
| `color` | color | `"#000000"` |
| `font_family` | string | `"sans-serif"` |

Built-in per-level sizes (used when a level has no `[heading.levels.N]`
override): H1 `28.0`, H2 `22.0`, H3 `18.0`, H4 `16.0`, H5 `14.0`, H6
`12.0`.

### `[heading.levels.<1-6>]`

| Field | Type | Default |
|---|---|---|
| `size_pt` | number | that level's built-in size |
| `color` | color | `[heading]`'s own `color` |
| `font_family` | string | `[heading]`'s own `font_family` |

## `[blockquote]`

| Field | Type | Default |
|---|---|---|
| `border_color` | color | `"#b4b4b4"` |
| `border_width_pt` | number | `2.0` |
| `indent_pt` | number | `18.0` |

## `[thematic_break]`

| Field | Type | Default |
|---|---|---|
| `color` | color | `"#c8c8c8"` |
| `width_pt` | number | `1.0` |

## `[list]`

| Field | Type | Default |
|---|---|---|
| `indent_pt` | number | `18.0` |

## `[table]`

| Field | Type | Default |
|---|---|---|
| `cell_padding_pt` | number | `12.0` |
| `text_size_pt` | number | `10.5` |
| `min_row_height_pt` | number | `20.0` |

## `[code_block]`

| Field | Type | Default |
|---|---|---|
| `syntax_theme` | string (a syntect theme name) | `"InspiredGitHub"` |
| `label_style` | `"none"` \| `"corner"` \| `"header_bar"` \| `"inline"` | `"none"` |
| `default_label` | string | `"text"` |

### `[code_block.default]`

| Field | Type | Default |
|---|---|---|
| `background` | color | `"#f5f5f5"` |
| `font_family` | string | `"monospace"` |
| `font_size_pt` | number | `10.0` |
| `label_color` | color | `"#666666"` |
| `label_background` | color | `"#e0e0e0"` |

### `[code_block.languages.<name>]`

`<name>` matches a fence's own language tag (e.g. `rust` for ` ```rust `).
Every field is optional and falls back to `[code_block.default]`:

| Field | Type |
|---|---|
| `label` | string |
| `background` | color |
| `font_family` | string |
| `font_size_pt` | number |
| `label_color` | color |
| `label_background` | color |

## `[header]` and `[footer]`

Identical schema, applied independently to the running header and footer.

| Field | Type | Default |
|---|---|---|
| `enabled` | boolean | `false` |
| `font_family` | string | `"sans-serif"` |
| `font_size_pt` | number | `9.0` |
| `color` | color | `"#666666"` |
| `mode` | `"uniform"` \| `"two_sided"` | `"uniform"` |
| `suppress_on_chapter_start` | boolean | `true` |

### `[header.uniform]` / `[footer.uniform]`, `.odd`, `.even`

Each is a set of three template strings, all defaulting to `""` (empty —
nothing shown in that zone):

| Field | Type | Default |
|---|---|---|
| `left` | template string | `""` |
| `center` | template string | `""` |
| `right` | template string | `""` |

`.uniform` is used when `mode = "uniform"`; `.odd`/`.even` are used when
`mode = "two_sided"`, selected by physical page parity. Valid placeholders
in a template string: `{page}`, `{total_pages}`, `{h1}`, `{h2}`, `{title}`,
`{author}`, `{date}` — see
[Headers and Footers](./styling/headers-and-footers.md#zones-and-templates).

## `[toc]`

| Field | Type | Default |
|---|---|---|
| `enabled` | boolean | `false` |
| `depth` | integer (`1`-`6`) | `2` |
| `title` | string | `"Table of Contents"` |
