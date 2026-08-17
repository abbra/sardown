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
