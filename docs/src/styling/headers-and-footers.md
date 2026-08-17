# Headers and Footers

Running headers and footers are both configured the same way, under
`[header]` and `[footer]` respectively — both disabled by default.

```toml
[footer]
enabled = true
font_family = "sans-serif"
font_size_pt = 9.0
color = "#666666"
mode = "uniform"
suppress_on_chapter_start = true

[footer.uniform]
center = "{page}"
```

## Zones and templates

Each of `uniform`, `odd`, and `even` (see [Two-sided mode](#two-sided-mode)
below) is a set of three independent template strings — `left`, `center`,
`right` — positioned across the content width. Leave a zone empty (the
default) to show nothing there.

A template can mix literal text with placeholders:

| Placeholder | Expands to |
|---|---|
| `{page}` | The current page number, formatted per `[page.numbering]` |
| `{total_pages}` | The document's total page count |
| `{h1}` | The most recent level-1 heading text before this page |
| `{h2}` | The most recent level-2 heading text before this page |
| `{title}` | `[document].title`, or the `--title` CLI flag if given |
| `{author}` | `[document].author`, or the `--author` CLI flag if given |
| `{date}` | `[document].date`, the `--date` CLI flag, or today's date if neither is set |

`{title}`/`{author}`/`{date}` come from a `[document]` section:

```toml
[document]
title = "My Book"
author = "Jane Doe"
date = "2026-01-01"   # a literal string, used as-is -- omit it to use today's date instead

[header]
enabled = true
uniform.center = "{title}"
```

`--title`/`--author`/`--date` on the command line override
`[document].title`/`.author`/`.date` if both are given — see the
[Command-Line Reference](../cli-reference.md). Unlike title/author,
`{date}` never renders empty: if neither the stylesheet nor `--date` sets
one, it defaults to today's date (in the local system's UTC day) at
render time.

```toml
[footer.uniform]
left = "{h1}"
right = "Page {page} of {total_pages}"
```

An unknown placeholder (a typo, or a `{` with no matching `}`) is a
load-time error naming the exact bad token — not a silently-broken
template or a runtime failure partway through rendering.

Only the first line of a resolved template is used: zones are expected to
be short enough not to wrap. If a template resolves to something long
enough to wrap, only its first line is shown, rather than overlapping
wrapped lines on top of each other.

## Two-sided mode

```toml
[footer]
mode = "two_sided"

[footer.odd]
right = "{page}"

[footer.even]
left = "{page}"
```

`mode = "uniform"` (default) uses the same `[footer.uniform]` zones on
every page. `mode = "two_sided"` uses `[footer.odd]` on odd physical pages
and `[footer.even]` on even ones — the conventional layout for a printed,
bound book where the outer margin (and therefore where the page number
should sit) alternates from page to page.

## Suppressing on chapter-opening pages

`suppress_on_chapter_start` (default `true`) hides the header/footer on any
page where a level-1 heading lands as the very first thing on that page —
matching the common printed-book convention of leaving the running
header/footer off a chapter's own opening page. Set it to `false` if you
want the header/footer to show on every page unconditionally (a common
choice for a single flat document like a business letter or academic paper,
which isn't structured into book-style chapters in the first place).

## Layout

Header/footer text is placed inside the page's existing margin, not in
extra space added beyond it — a stylesheet doesn't need to separately
account for header/footer height when setting `[page].margin_mm`.
