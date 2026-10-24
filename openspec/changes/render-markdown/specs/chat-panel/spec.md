# Spec Delta

## ADDED Requirements

### Requirement: Replies render a markdown subset

An answer's text SHALL be rendered as markdown rather than drawn literally, for
this subset: fenced code blocks, inline code, strong and emphasis, headings,
unordered and ordered lists, block quotes, and the text of links. Everything
outside the subset — tables, task lists, raw HTML, footnotes, strikethrough — SHALL
be drawn as written, never dropped and never turned into something else.

A fenced code block SHALL be set in a monospace face at the same size and on the
same row grid as the rest of the answer, wrapped, with a hanging indent and a
tinted background, and SHALL NOT be syntax highlighted, labelled with its language,
or scrolled horizontally.

A list SHALL be indented by nesting level, a heading SHALL be bold and preceded by
one empty row, and a block quote SHALL be indented behind a rail. A link SHALL show
its text and not its URL.

The rendered text of a row SHALL be exactly the characters the reader sees: the
markup itself SHALL NOT be drawn, and SHALL NOT be part of what a selection or a
copy takes. A selection, a copy and the copy-mode cursor SHALL address characters of
that visible text, so copying a code block yields the code and nothing else.

#### Scenario: A code block is code

- **WHEN** an answer contains a fenced block of aligned text
- **THEN** its columns stay aligned in a monospace face on a tinted background, a
  long line wraps inside the block with a hanging indent, and no backticks or
  language label are drawn

#### Scenario: Markup is not text

- **WHEN** an answer contains `**bold**` and is copied
- **THEN** the reply is drawn bold and the clipboard holds `bold`, without the asterisks

#### Scenario: What is not supported stays readable

- **WHEN** an answer contains a markdown table or raw HTML
- **THEN** those characters are drawn as written, and no text is lost

#### Scenario: An answer arriving mid-structure

- **WHEN** a stream stops in the middle of a fence or an emphasis
- **THEN** what arrived is drawn as the block it is heading towards, and nothing is
  dropped or duplicated when the rest arrives
