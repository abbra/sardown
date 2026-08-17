# Headings

```toml
[heading]
space_before_factor = 0.8   # default
color = "#000000"
font_family = "sans-serif"
```

`color` and `font_family` apply document-wide, to every heading level, and
can each be overridden for a specific level (see below). `space_before_factor`
scales the vertical gap inserted before a heading, proportional to that
heading's own font size — a larger factor means more breathing room above
each heading. It has no effect on a heading that lands as the very first
thing on a page (no extra gap is added there).

## Built-in sizes

Without any per-level override, headings use this default size table:

| Level | Size |
|---|---|
| H1 | 28pt |
| H2 | 22pt |
| H3 | 18pt |
| H4 | 16pt |
| H5 | 14pt |
| H6 | 12pt |

## Per-level overrides

```toml
[heading.levels.1]
size_pt = 32.0
color = "#0b3d66"

[heading.levels.2]
font_family = "Georgia"
```

Each level (`1` through `6`) can override `size_pt`, `color`, and/or
`font_family` independently. Anything not overridden for a given level
falls back to that level's built-in size (from the table above) and to
`[heading]`'s own document-wide `color`/`font_family`.

## Note on `[heading.levels]` and TOML

Because TOML tables are replaced wholesale rather than merged key-by-key
when deserialized, setting `[heading.levels.1]` only affects level 1 —
every other level still falls back to the built-in size table and the
document-wide color/font, exactly as if `[heading.levels]` had been left
out entirely. You don't need to (and shouldn't) repeat unrelated levels
just because you're overriding one.
