# Spec Delta

## MODIFIED Requirements

### Requirement: Replies render a markdown subset

An answer's text SHALL be rendered as markdown rather than drawn literally, for
this subset: fenced code blocks, inline code, strong and emphasis, headings,
unordered and ordered lists, block quotes, horizontal rules, tabs, and the text of
links. Everything outside the subset — tables, task lists, raw HTML, footnotes,
front matter, strikethrough, math — SHALL be drawn as written, never dropped and
never turned into something else.

A fenced code block SHALL be set in a monospace face at the same size and on the
same row grid as the rest of the answer, wrapped, with a hanging indent and a
tinted background, and SHALL NOT be syntax highlighted, labelled with its language,
or scrolled horizontally. An inline code span SHALL be set in the same monospace face
and SHALL carry a chip behind it, so a code word inside a sentence reads as code.

Emphasis SHALL be drawn in a slanted face when the system has one, and otherwise in
an ink of its own; a synthetic slant MUST NOT be used. Styles SHALL compose, so
emphasis inside strong is both, and a code span inside a link is both.

A list SHALL be indented by nesting level, a heading SHALL be bold and preceded by
one empty row, and a block quote SHALL be indented behind a rail. A link SHALL show
its text and not its URL, and SHALL carry an underline under that text. A horizontal
rule SHALL be drawn as one hairline across the card's text width, not as an empty row.

A tab SHALL be expanded to the next multiple of four columns when the answer is laid
out, so the width that was measured is the width that is drawn.

The rendered text of a row SHALL be exactly the characters the reader sees: the
markup itself SHALL NOT be drawn, and SHALL NOT be part of what a selection or a
copy takes. A selection, a copy and the copy-mode cursor SHALL address characters of
that visible text, so copying a code block yields the code and nothing else.

A row with no text SHALL be stepped over by the keyboard's line movement and by the
copy-mode motions, in either direction, while a copy that spans rows SHALL keep the
empty rows between them.

#### Scenario: Emphasis inside strong

- **WHEN** an answer contains `**bold with *italic* inside**`
- **THEN** the words are bold, the inner phrase is also slanted, and a copy of the line
  contains neither asterisk

#### Scenario: A code word in a sentence

- **WHEN** an answer mentions `` `EAGAIN` `` mid-sentence
- **THEN** it is drawn in the monospace face on a chip, and the sentence's wrapping
  accounts for the width it actually takes

#### Scenario: A rule

- **WHEN** an answer contains `---` between two paragraphs
- **THEN** a hairline is drawn across the card, and neither dashes nor a blank row are
  left as the only trace

#### Scenario: A tab in code

- **WHEN** a fenced block indents with tabs
- **THEN** each tab advances to the next multiple of four columns, and the row ends
  inside the card

#### Scenario: The keyboard does not stop on a blank row

- **WHEN** `Down` is pressed and the next row is the empty row between two blocks
- **THEN** the highlight lands on the next row that has text

#### Scenario: A face the system does not have

- **WHEN** an answer contains emphasis and the system has no italic face
- **THEN** the emphasis is still distinguished, and the answer is drawn without a
  synthesised slant
