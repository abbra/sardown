# Cross-References

Relative Markdown links between chapters become working internal PDF
links (jump-to-page links within the same PDF), not dead relative-path
references.

## Linking to another chapter

```markdown
See the [installation guide](installation.md) for setup steps.
```

If `installation.md` is a chapter listed anywhere in `SUMMARY.md`, this
link resolves to that chapter's first heading — landing the reader at the
top of that chapter when clicked.

## Linking to a specific heading

```markdown
See [Custom Fonts](installation.md#custom-fonts) for details.
```

Fragment links resolve against the *target chapter's own* heading anchors
(auto-generated slugs from heading text), not the linking chapter's — so
two different chapters can each have a heading that happens to slugify to
the same text (e.g. both have an "Overview" section) without your link
accidentally landing in the wrong one.

## What counts as a chapter link

Only links that resolve (relative to the linking chapter's own directory)
to a file actually listed in `SUMMARY.md` are turned into internal links.
A relative link to a file that exists on disk but *isn't* a listed chapter
is left exactly as written (an inert, non-clickable relative path) — it's
not classified as a cross-reference. True absolute URLs
(`https://example.com/...`) are never touched.

## Unresolvable fragments

A link like `chapter.md#no-such-heading`, where `chapter.md` is a real,
listed chapter but no heading in it actually slugifies to
`no-such-heading`, is recognized as an attempted cross-reference but can't
be resolved — it falls back to inert (unlinked) text rather than pointing
at a broken destination.

## What cross-references can't do

Everything above produces a **clickable link** — a reader clicking it
jumps to the target. There's no way to resolve a cross-reference into
**inline text naming its page number** (the "see page 42" convention
common in printed books, as opposed to a clickable link). If you write
"see the installation guide (page 42)" the "42" is just literal text you
typed — it's never checked or kept in sync with where that chapter
actually ends up landing after layout.
