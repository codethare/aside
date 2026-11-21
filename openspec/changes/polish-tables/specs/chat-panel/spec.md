# Spec Delta

## MODIFIED Requirements

### Requirement: Replies render a markdown subset

An answer's text SHALL be rendered as markdown rather than drawn literally, for
this subset: fenced code blocks, inline code, strong and emphasis, headings,
unordered and ordered lists, block quotes, GFM tables, and the text of links.
Everything outside the subset — task lists, raw HTML, footnotes, strikethrough,
math — SHALL be drawn as written, never dropped and never turned into something
else.

A fenced code block SHALL be set in a monospace face at the same size and on the
same row grid as the rest of the answer, wrapped, with a hanging indent and a
tinted background, and SHALL NOT be syntax highlighted, labelled with its language,
or scrolled horizontally.

A table SHALL be drawn with its columns padded so that every row of the table starts
its columns at the same place, in the monospace face, with one space between
columns. Its header SHALL be followed by one rule row. Column alignment SHALL be
honoured. A cell too wide for its column SHALL wrap inside it and its row SHALL grow
to the tallest cell; a row with fewer cells than the header SHALL be padded.

Column widths SHALL be measured by display width in the code face, not by character
count: a CJK glyph or an emoji takes the advance its face gives it, a combining mark
adds none, and a character no face covers advances by one space. The header SHALL
participate in the widths. A table SHALL not be drawn until its header and separator
have arrived; until then the text is the paragraph it currently is.

The header SHALL start the column widths, fitted to the card once if it is too wide.
Each later row MAY widen a column but SHALL NOT narrow one: units are added only
while the card has room, spread around the row, so a table that grows while an answer
streams does not move its columns left. When the answer completes, its open tail MAY
be laid out once more with the final natural widths; no further row can then move it.

A table whose minimum columns and gaps cannot fit the card SHALL degrade to wrapped
`header: value` lines, one per body cell, and SHALL NOT lose its right-hand columns
to clipping.

The rendered text of a row SHALL be exactly the characters the reader sees: the
markup itself SHALL NOT be drawn, and SHALL NOT be part of what a selection or a
copy takes. A selection, a copy and the copy-mode cursor SHALL address characters of
that visible text, so copying a code block yields the code and nothing else.

A table row SHALL be copied as TSV: the cells without the screen's padding, joined by
tabs. Each display row of a wrapped table row SHALL know its logical row and cells;
a selection over several display rows of one cell SHALL copy that row once. A
selection that touches a table row SHALL mark the whole display row, and every
display row of that logical row, rather than half a cell. Copying a non-table row is
the visible characters, as before.

#### Scenario: A code block is code

- **WHEN** an answer contains a fenced block of aligned text
- **THEN** its columns stay aligned in a monospace face on a tinted background, a
  long line wraps inside the block with a hanging indent, and no backticks or
  language label are drawn

#### Scenario: A table is a table

- **WHEN** an answer contains a GFM table
- **THEN** its header is drawn on one row with a rule under it, its columns start at
  the same place on every row, `:---:` and `---:` align their columns, a long cell
  wraps and its row grows, and no pipe or dash from the source is drawn

#### Scenario: CJK and emoji cells

- **WHEN** a table has CJK or emoji cells beside Latin ones
- **THEN** the following column starts at the same display x on every row, because
  the width is measured, not counted

#### Scenario: A table grows while the answer streams

- **WHEN** a new, longer body row arrives
- **THEN** columns may widen and never narrow, and the header's column starts do not
  move left

#### Scenario: A table wider than the card

- **WHEN** even the minimum columns and gaps do not fit the card
- **THEN** each body cell is drawn as a wrapped `header: value` line, with no
  right-hand column clipped

#### Scenario: Copying a table row

- **WHEN** a table row is copied
- **THEN** the clipboard holds the cells separated by tabs, without the visual
  padding, and a wrapped cell is copied once

#### Scenario: Markup is not text

- **WHEN** an answer contains `**bold**` and is copied
- **THEN** the reply is drawn bold and the clipboard holds `bold`, without the asterisks

#### Scenario: What is not supported stays readable

- **WHEN** an answer contains a task list or raw HTML
- **THEN** those characters are drawn as written, and no text is lost

#### Scenario: An answer arriving mid-structure

- **WHEN** a stream stops in the middle of a fence or an emphasis
- **THEN** what arrived is drawn as the block it is heading towards, and nothing is
  dropped or duplicated when the rest arrives
