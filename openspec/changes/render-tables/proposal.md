# Render markdown tables

## Why

Tables are the one thing a model reaches for that the panel still hands back as raw
characters: `| a | b |` and its `|---|` rule arrive as text, so a three-column
comparison reads as a wall of pipes. `render-markdown` kept them out deliberately —
the row model is one line of text per row, and a table is two dimensions. It is the
last block mark in the subset that a reader notices within one screen.

## What changes

GFM tables render as a table: a header row, a hairline under it, and one line per
physical row of the table's body, with the columns padded so they line up. Column
alignment (`:---`, `:---:`, `---:`) is honoured. A cell whose text does not fit its
column wraps inside it, and the row grows to its tallest cell. A row with fewer
cells than the header is padded. A table too wide for the card has its widest
columns shrunk until it fits.

Everything about a table is drawn in the **monospace face**, because that is what
makes a column a column; a proportional face has no two characters the same width
and padding with spaces would not line anything up.

This is the only new parser option: `Options::ENABLE_TABLES`. Task lists,
footnotes, strikethrough, math and raw HTML stay as written.

## What does not change

- The row model, the grid and the offsets: a table row is still a `Line` with a
  plain visible string, so a selection, a copy, the highlight and the copy-mode
  cursor keep addressing characters, and a copy of a table row keeps its column
  padding — because that padding is what the reader sees.
- `boundary()` needs no change: a GFM table has no blank line inside it, so a table
  stays in the open block until a blank line ends it, exactly like a paragraph.
- Nothing is written anywhere; a table is laid out in memory like everything else.

## Cost

`src/markdown.rs` grows a table collector and a column fitter, roughly 180 lines,
plus the tests. Per delta the cost stays the open block's: a table re-fits when it
grows, and a long table arriving in chunks re-fits per chunk, bounded by
`REPLY_CAP`.

## Impact

- `src/markdown.rs`: `Table`/`Cell` collection from the parser's table events, the
  column fitter, cell wrapping, padding and alignment, the header rule.
- `specs/chat-panel`: the subset requirement gains tables and loses them from the
  "as written" list.
- `README`, `docs/aside.1`, `CLAUDE.md` known limits, `docs/live-checklist.md`.
- No dependency change; `pulldown-cmark` already has the parser.
