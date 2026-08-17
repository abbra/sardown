# Rendering Books

`md2pdf render-book <book_root> -o <output.pdf>` renders an entire
[mdBook](https://rust-lang.github.io/mdBook/)-style source tree as one
combined PDF, rather than one file at a time.

## Expected directory layout

```
my-book/
├── book.toml       # optional
├── style.toml      # optional -- see below
└── src/
    ├── SUMMARY.md
    ├── chapter-1.md
    └── chapter-2.md
```

- `book.toml` is optional. If present, `[book] src = "..."` overrides the
  source directory name (default `"src"`). Everything else in `book.toml`
  is ignored by md2pdf today.
- `src/SUMMARY.md` is required — see
  [SUMMARY.md Format](./summary-format.md) for its exact structure.
- `style.toml`, if present at the book's root, is used automatically
  *unless* `--style <path>` is given explicitly on the command line. See
  [Styling Your Documents](../styling/index.md).

## What you get

- Every chapter listed in `SUMMARY.md`, concatenated in listing order
  (depth-first, following nested chapters), each starting on its own new
  page.
- If a chapter file has no top-level (`#`) heading of its own, one is
  synthesized from its `SUMMARY.md` title, so every chapter has *something*
  identifying it at the top of its page.
- Relative Markdown links between chapters resolved into working internal
  PDF links — see [Cross-References](./cross-references.md).
- Each chapter's own embedded images resolved relative to *that chapter's*
  directory, so chapters in different subdirectories can each have their
  own `images/` folder without collision.
- Mermaid diagram failures reported with the chapter's own path, not an
  opaque internal ID — see [Diagrams](../diagrams.md).

## What's not yet supported

- `\{{#include ...}}` and other mdBook preprocessor directives are not
  processed — chapter files are read as plain Markdown.
- A table of contents page is not generated automatically.
- `PartTitle` entries (headings in `SUMMARY.md` used as sidebar group
  labels in mdBook's own HTML output) are recognized during parsing but
  currently have no visual treatment in the combined PDF body.
- mdBook's "prefix chapter" convention (a bare link listed before the
  first list item, outside any list) isn't recognized — see
  [Prefix chapters aren't supported](./summary-format.md#prefix-chapters-arent-supported).
