# Design

## The row model is one line of text, so a table is built out of lines of text

A `Line` is a plain visible string with style spans over it, drawn at one `row_h`.
A table is therefore not a grid object: it is a sequence of lines whose strings
happen to contain padded columns. Everything downstream — the grid, the viewport,
the selection band, the copy — keeps working untouched, which is the whole reason
to build it this way.

## Everything is monospace, and padding is counted in spaces

Columns line up only if the same string prefix has the same width on every row. With
a proportional face that is false, so table rows are drawn in the code face
(`Style::Code`, `font.mono()`), the same face a fenced block uses. If the system has
no monospace face, the table still renders in the text face and the columns will be
ragged — better than not rendering it.

The unit is the space's advance in the code face:

```
unit = ceil(advance(mono, " "))            // at least 1 px
units(text) = ceil(advance(mono, text) / unit)
```

A column's width is an integer number of units. Every cell is padded with spaces to
that same count, so two rows agree on where a column starts — exactly, because the
padding is spaces and a space is the unit.

## Fitting the card

1. `width[i]` starts at the widest cell in column `i`, in units, floored at
   `MIN_UNITS = 3`.
2. `total = sum(width) + (columns - 1)` for the single-space gap between columns.
3. `avail = floor((max_w - indent) / unit)`. While `total > avail`, shrink the
   currently widest column by one unit if it is above `MIN_UNITS`; when no column
   can shrink, stop and let the painter clip the row at the card's edge. A table
   that cannot be shrunk is drawn whole and cut, never reflowed into something the
   model did not write.
4. A cell is wrapped to `width[i] * unit` pixels with the same wrapper a paragraph
   uses, measured in the code face.

## A logical row is as tall as its tallest cell

Each cell wraps to a list of lines; the row's height is the longest list. Cells with
fewer lines contribute empty strings for the extra rows, so the columns below stay
under themselves. The header is a logical row like any other.

## Marks

- The header row is followed by one rule row: `─` repeated to the table's total
  width, in the dim ink (`FG_NORMAL`), drawn in the code face so the line is
  continuous. It is a character, not a hairline primitive: the row model has no way
  to draw a rule, and a `─` row needs nothing new.
- The header is not set in a different weight. The current span model holds one
  style, so "monospace and bolder" cannot be said; the rule is what marks the
  header. (When the `Styles` set from `complete-the-markdown-rendering` lands, the
  header can take strong plus code.)
- Alignment pads the short side: `None` and `Left` pad on the right, `Right` pads on
  the left, `Center` splits the remainder, the extra unit going right.

## Indentation and nesting

`indent` and `quote` come from the same place every other block gets them, so a
table inside a list or a quote steps in and carries the rail like a paragraph would.

## What a copy takes

The row's own string, padded. The invariant is that a row's text is exactly the
characters a reader sees; a table's columns are those characters. Copying a table
row therefore yields `a    b    c` with the alignment spaces, not a tab-separated
line.

## Failure modes

- A table with no header separator is not a table to GFM and never reaches the
  renderer.
- A cell holding an inline style keeps the span; `Style::Code` covers the whole row
  (padding included), so a `**strong**` inside a cell is drawn in the table's ink.
  This is the same single-style limitation the header hits, and it goes away with
  `Styles`.
- A row wider than the card after shrinking is clipped by the painter, like any
  other over-long row.
