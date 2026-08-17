# Typography

```toml
[typography]
font_family = "sans-serif"   # default
font_dirs = []
use_system_fonts = true
body_size_pt = 12.0
body_color = "#000000"
alignment = "left"
hyphenation = false
language = "en-us"
```

This controls ordinary body text: paragraphs, list items, and blockquote
text. (Headings have their own font/color/size settings — see
[Headings](./headings.md) — and table cells reuse `font_family` from here
but have their own text size — see [Tables](./tables.md).)

## Font family

`font_family` accepts either a generic keyword — `"serif"`, `"sans-serif"`
(the default), `"monospace"`, `"cursive"`, `"fantasy"` — or a literal font
name like `"Times New Roman"` or `"Georgia"`. A literal name only works if
that font is actually loadable (installed as a system font, or found in
one of `font_dirs`); see
[Font Resolution](../troubleshooting.md#unknown-font-family-warnings) for
what happens when it isn't.

## Font discovery

- `use_system_fonts` (default `true`): scan the machine's installed
  system fonts.
- `font_dirs`: a list of additional directories to scan for font files,
  useful for bundling a specific font alongside your document instead of
  depending on it being installed system-wide.

## Body size and color

`body_size_pt` is the font size in points (default `12.0`). `body_color`
accepts either a 6-digit hex string (`"#1a1a1a"`, with or without the
leading `#`) or an explicit `[r, g, b]` array (`[26, 26, 26]`) — use
whichever is more natural for a given value; both forms are accepted
everywhere a color field appears in a stylesheet.

## Alignment

`alignment` controls how paragraph, list, and blockquote text wraps:

- `"left"` (default) — ragged-right, the conventional choice for English
  and most Latin-script typesetting.
- `"justify"` — flush on both sides (each wrapped line, except a
  paragraph's last line, stretches to fill the full content width),
  the conventional choice for Russian and much continental European
  book typesetting.
- `"right"`, `"center"` — also available, less commonly needed for body
  text.

Headings, code blocks, and table cells are always left-aligned regardless
of this setting.

## Hyphenation

```toml
[typography]
hyphenation = true
language = "en-us"
```

`hyphenation` (default `false`) turns on real, dictionary-based
hyphenation for paragraph, list-item, and blockquote text — most useful
with `alignment = "justify"`, where a long word that can't split leaves
the words around it stretched further apart than necessary. `language`
selects which hyphenation patterns to use; it accepts any of the ~79
BCP-47-ish codes the `hyphenation` crate embeds (`"en-us"`, `"en-gb"`,
`"de-1996"`, `"fr"`, and so on — see
[its documentation](https://docs.rs/hyphenation/latest/hyphenation/enum.Language.html)
for the full list). An unrecognized `language` value disables hyphenation
for the render with a warning, the same way an unresolvable `font_family`
falls back with a warning rather than failing the render.

Headings, code blocks, and table cells are never hyphenated, regardless
of this setting.

**Current limitation:** a word carrying any punctuation, digits, or an
apostrophe (`"don't"`, `"word,"`, `"word."`) is never hyphenated in the
current implementation — only whole alphabetic words are considered. This
still covers the common case (long technical or compound words), but a
word at the end of a sentence or clause won't split even if it's long
enough to benefit.
