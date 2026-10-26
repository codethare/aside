# Complete the markdown rendering

## Why

The renderer that shipped with `render-markdown` handles the block structure and
keeps the invariant that matters — a row's text is exactly the characters a reader
sees, so selection and copy never take markup. What it does not yet do is the part
of markdown a reader notices within a screen:

- `*italic*` is drawn in the same ink, in the same face, as plain text: emphasis is
  invisible. The known-limits list admits it; nobody reading an answer would.
- `**a *b* c**` loses its italic: a span holds **one** style, so the inner emphasis
  is dropped rather than nested.
- Inline code is monospace and brighter but has no chip behind it, so a one-word
  `EAGAIN` in a sentence reads as a font accident rather than as code.
- A link's text is indistinguishable from the prose around it, and `---` becomes a
  blank row — the one place the renderer draws something other than the characters
  or a mark for the structure.
- A tab has no glyph, so it is measured as half an em by the wrapper and drawn as
  nothing: one tab per row is enough to push a row past the card's edge.
- The highlight and the copy-mode cursor stop on the empty rows the renderer makes,
  so `Up`/`Down` can land on a row with no text in it.

## What changes

Five phases, each with its own tests. Nothing here changes the row model, the grid,
or the offsets the selection and the copy use.

### 1. Inline styles are a set, not a choice

`Span.style` becomes a small set of flags — strong, emphasis, code, link — so
emphasis inside strong survives, and so a code span inside a link keeps both facts.
The painter composes them: strong and code take the brighter ink, emphasis takes the
slanted face, a link takes the underline, and code takes the chip.

### 2. A slanted face for emphasis

`PanelFont` gains an italic chain beside the monospace one, read the same lazy way
and found the same way: a candidate path list first, the font database afterwards,
and `None` when the system has no italic face — in which case emphasis is drawn in
its own ink weight rather than a synthetic slant. The glyph cache already keys on
the face, so a second face costs nothing per frame.

### 3. Marks that are not text

- Inline code gets a chip: `BG_CODE` behind the span, two pixels of padding either
  side, drawn before the text like the selection band.
- A link gets a one-pixel underline under its own span, in the dim ink.
- `---` becomes a hairline row: one pixel across the card's text width, in the dim
  ink, which is what a rule means. (It is currently an empty row.)

### 4. Text the parser hands over verbatim

- Tabs are expanded to the next multiple of four at render time, in code and in
  prose, so what is measured is what is drawn.
- A closing fence may carry whitespace or a trailing run shorter than the opener and
  still close the block it opened; an opener indented up to three spaces still opens
  one.
- Anything else stays as written: tables, task lists, footnotes, front matter,
  strikethrough, math, HTML.

### 5. What the keyboard walks

The highlight and the copy-mode cursor skip a row that has no text — a blank row is
spacing, not a line of the answer — while a copy that spans rows still takes the
blank lines between them, because the answer had them.

### 6. Not in this change, and why

Two of these would cost the architecture rather than a feature. A row is
`entry.y + index * row_h`, and `row_y`, `row_at`, the viewport, the highlight, the
copy cursor and the block spacing are all built on that one number: anything whose
row is taller than a text row breaks all of them at once.

- **Tables** would need column widths measured across the whole table and cells
  wrapped inside a cell, so rows differ in height. The card is 640 px wide — about
  600 px of text, roughly twenty Latin characters a column — so cells wrap on any
  real table. *Cheap middle ground available*: draw a table as a monospace block,
  one row per source row, cells padded on the character grid, long cells ending at
  the card's edge. Paper quoted on request.
- **Proportional heading sizes** need a second size and a taller row: same break.

Three are dependency decisions, not rendering ones:

- **Syntax highlighting** needs a grammar set — `syntect`, or a `tree-sitter-*` per
  language — against a rule that allows seven dependencies. The panel also has three
  inks to spend; a theme would be a second decision on top.
- **Math** is only *tagged* by the parser: `ENABLE_MATH` hands over the TeX source.
  Drawing it needs a layout engine or glyph assembly, so enabling the option would
  change nothing a reader sees. It stays literal, as it is now.
- **Horizontal scrolling of code** was decided the other way when the scope was
  chosen: code wraps with a hanging indent. Scrolling would give a code row a second
  x-origin beside its indent, and every character-to-pixel question — the selection,
  the copy cursor, the chip widths — would have to say which origin it means.

Two would be dishonest in a read-only panel:

- **Task lists** draw checkboxes nobody can tick. `- [ ]` already reads as a bullet
  and the characters, which is true.
- **Footnotes** need a raised marker in a second size and an appendix at the end of
  the answer, which in a thirteen-row card means jumping the view and coming back.

One is absent from the input: **front matter** describes a document, not an answer,
and turning it on would make a leading `---` disappear as metadata instead of being
drawn as the rule it is.

One is cheap and stays out only to keep the marks phase small: **strikethrough** is
one option flag and one line through the span once the underline exists — about
fifteen lines. Say so and it joins phase 3.

## Cost

- One new face in the chain (a lazy read, like the monospace one).
- The span model widens from one byte to four; the painter composes them in one
  place, `ink_of`/`face_of` become one `look_of`.
- Chips and underlines are rectangles drawn per span that needs one — a handful per
  screen, not per row.
- No dependency, no configuration, nothing written to disk.

## Impact

- `src/layout.rs`: `Span` carries the set; `style_at` becomes `styles_at`.
- `src/markdown.rs`: nesting, tab expansion, fence edges, the rule row.
- `src/panel.rs`: chips, underlines, the hairline, the composed look.
- `src/font.rs`: the italic chain.
- `src/transcript.rs` + `src/main.rs`: navigation skipping empty rows.
- `README`, `docs/aside.1`, `CLAUDE.md` known limits: emphasis no longer listed as
  unstyled, `---` no longer listed as a blank row.
