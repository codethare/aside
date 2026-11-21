# Make tables measure, grow and copy like data

## Why

The table renderer counts space units from the font, but it does not use the header
as the first width, and a new wide row can make the fitting pass take units back
from another column: a table in a streaming answer moves sideways as it grows. The
narrow fallback stops at `MIN_UNITS` and lets the painter clip the right-hand columns,
so a table that cannot fit silently loses data. A copied table row carries the spaces
that put the columns on screen, which paste into a spreadsheet as one cell.

The alignment rule and the header rule are already right; what is missing is a stable
width policy, a fallback that keeps every cell, and a copy that separates cells.

## What changes

- **Widths are display widths.** A column is measured by the code face's advance,
  not by `chars().count()`: CJK and emoji take the width their face gives them, a
  combining mark adds none, and a missing glyph advances by a space. The header is
  included in the measurement.
- **The header starts the widths; rows only grow them.** The header is fitted to the
  card once (widest column first, down to the minimum). Each body row then asks for
  more units and gets them only while the card has room, one unit at a time around
  the row. No later row can shrink a column, so a streaming table does not jitter.
  A table is not drawn until the header and its separator have arrived. When the
  answer completes, the open tail is fitted once more with its natural widths, so the
  finished table is not left narrower than it needs to be.
- **A narrow table degrades to pairs.** If even the minimum columns and gaps cannot
  fit, each body cell becomes `header: value`, wrapped to the card, instead of the
  right-hand columns being clipped.
- **A table row copies as TSV.** Each display row carries its logical row and the
  unpadded cells. A whole-line copy emits the cells separated by tabs; a selection
  copies each logical row once, even when a cell wraps over several display rows.
  The selection mark covers the whole display row for a table, never half a cell.

## Capabilities

### Modified Capabilities

- `chat-panel`: table widths, the narrow fallback, and table-row copying.

## Impact

- `src/markdown.rs`: the width policy, the pair fallback, and the table metadata.
- `src/layout.rs` and `src/transcript.rs`: a row carries its logical table cells.
- `src/main.rs`: copy text reads the cells as TSV.
- `src/panel.rs`: a table selection marks whole display rows.
- README and the man page.
