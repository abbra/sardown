# Table of Contents

```toml
[toc]
enabled = false   # default
depth = 2
title = "Table of Contents"
```

Enabling `[toc]` prepends a generated table-of-contents page (or pages,
for a long document) before your content, listing every heading at or
above `depth` (default `2`, meaning H1 and H2 -- most documents put both
in their table of contents). Each entry shows the heading text, a row of
dots, and its page number, and is a clickable link to that heading. The
PDF's own outline/bookmark panel (the navigation sidebar most PDF viewers
show) is populated from the same heading list.

`depth` accepts `1` through `6`. `title` is the heading shown at the top
of the generated page(s).

With no stylesheet at all (or `[toc].enabled` left at its default
`false`), no table of contents is generated -- this is an opt-in feature,
not automatic.

A document with no headings at or above `depth` gets no table-of-contents
page at all, rather than an empty one.
