# Tables

```toml
[table]
cell_padding_pt = 12.0    # default
text_size_pt = 10.5
min_row_height_pt = 20.0
```

- `cell_padding_pt` — horizontal padding inside every cell, subtracted
  from the column's own width when wrapping cell text (so text never
  bleeds into the grid line or the next column).
- `text_size_pt` — the font size used for all table cell text (headers and
  body cells alike). Table cells always use the body typeface from
  `[typography]`'s `font_family` — there's no separate table font setting.
- `min_row_height_pt` — the shortest a row will ever be, even if every
  cell's content is a single short word. A row with wrapped multi-line
  cell content grows taller than this automatically.

## Rules, fills and the header row

```toml
[table]
border = "horizontal"        # "grid" (default), "horizontal" or "none"
border_color = "#c5d6e0"
border_width_pt = 0.75
row_padding_pt = 3.0         # extra space above and below each row's text
header_background = "#e3eef5"
header_color = "#00527d"
header_bold = true
stripe_background = "#f3f8fb"  # every second body row
space_after_pt = 8.0         # gap before the next block
```

- `border = "grid"` draws a vertical line between columns and one under the
  header; `"horizontal"` draws rules above the table, under the header and
  under every row; `"none"` draws no rules.
- All fill and header options are off by default, so existing stylesheets
  render as before.
- In a slide deck, the padding, row height and `space_after_pt` shrink
  together with the text when a slide is scaled down to fit.

## Column widths and alignment

Column widths are computed automatically from content — there's currently
no stylesheet control over column width distribution. Column alignment
(left/center/right) comes from the Markdown table's own alignment markers
(`:---`, `:---:`, `---:`), not from the stylesheet — see
[Writing Markdown](../markdown-support.md#tables).

## Pagination

A table can span multiple pages; rows are never split across a page break
(a row that doesn't fit moves to the next page as a whole), and grid lines
are drawn separately per page segment.
