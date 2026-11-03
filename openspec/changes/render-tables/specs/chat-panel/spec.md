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
to the tallest cell; a row with fewer cells than the header SHALL be padded; a table
too wide for the card SHALL have its widest columns narrowed until it fits, and MUST
NOT be broken into a different structure.

A list SHALL be indented by nesting level, a heading SHALL be bold and preceded by
one empty row, and a block quote SHALL be indented behind a rail. A link SHALL show
its text and not its URL.

The rendered text of a row SHALL be exactly the characters the reader sees: the
markup itself SHALL NOT be drawn, and SHALL NOT be part of what a selection or a
copy takes. A selection, a copy and the copy-mode cursor SHALL address characters of
that visible text, so copying a code block yields the code and nothing else, and
copying a table row yields the padded row as drawn.

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

#### Scenario: A table wider than the card

- **WHEN** a table's columns do not fit the card's width
- **THEN** its widest columns are narrowed until it fits and every row still starts
  its columns at the same place

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
