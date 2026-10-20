# Spec Delta

## MODIFIED Requirements

### Requirement: The panel is a layer surface in three states

The panel SHALL be a `wlr-layer-shell` surface on the overlay layer, anchored to
the top edge with a 32 px margin, horizontally centred on its output, with
exclusive keyboard interactivity. It SHALL have exactly three states:

| State | Panel size (logical px, 1×) | Content |
|---|---|---|
| idle | 640 × 30 | the input bar alone |
| thinking | 60 × 60 | a filled circle with four blinking squares |
| answering | 640 × (bar_h + divider + transcript height), capped | the bar on top, the transcript below it |

`row_h` is `row_h_for(20)` (30 at 1×). The transcript height is the sum of its
blocks' heights — each block being one text row, or a card's text rows plus its
padding — together with the gap between blocks, and it SHALL NOT exceed the
height of 64 text rows. A size change SHALL be committed before the frame that
uses it. These are the panel's sizes: the surface carries the frame margin below,
so its size is `panel + 2 × 24` on each axis and its top margin is 8 px — the
panel keeps a 32 px visible gap to the screen edge.

#### Scenario: Summoned idle

- **WHEN** the panel opens with nothing asked
- **THEN** only the bar is on screen: a 640 × 30 panel, no transcript area, inside a 688 × 78 surface

#### Scenario: Enter starts the ball

- **WHEN** the user presses `Enter` with a non-empty input
- **THEN** the request starts and the panel becomes a 60 × 60 square centred on the bar, painting a circle

#### Scenario: The first delta expands the panel

- **WHEN** the answer's first text arrives
- **THEN** the panel returns to 640 wide, keeps the bar as its top row, and grows by the height the answer's card adds

#### Scenario: Typing during the ball is ignored

- **WHEN** the panel is thinking and a printable key is pressed
- **THEN** the input is unchanged, and `Esc` still cancels the request

### Requirement: The frame matches wlmenu's frame

The frame and the bar SHALL be drawn exactly as `wlmenu` draws them: a
premultiplied translucent body (`BG_NORMAL` at 83 % alpha, `BG_PROMPT` at 86 %),
a 24 px shadow margin around the panel holding both drop-shadow layers, a
one-pixel hairline border lit at the top and dark underneath, antialiased
corners, the bar's 4 px `LABEL_ACCENT` rail, 8 px gap and 1 px divider inset by
`PAD`, and a 3 px `LABEL_ACCENT` scroll indicator at the right edge while the
transcript overflows the viewport. The thinking ball SHALL use the same body,
border and shadow inside its circle.

Inside the transcript the panel SHALL NOT imitate `wlmenu`'s list rows: there
SHALL be no full-width selection band and no uniform row rhythm, because the
transcript is a stack of blocks with a container of their own (see below).

#### Scenario: One panel, two products

- **WHEN** a screenshot of `aside`'s idle panel and one of `wlmenu` are laid side by side
- **THEN** their shadow falloff, body translucency, border, corner shape, rail, divider and inset agree

#### Scenario: A long transcript scrolls with an indicator

- **WHEN** the transcript is taller than the capped panel
- **THEN** a 3 px `LABEL_ACCENT` thumb sits at the right edge, its length in proportion to the visible height

### Requirement: The transcript scrolls with flat frame cost

The panel SHALL show a bounded viewport of at most 64 text rows and support
scrolling through a transcript far larger than that. The viewport SHALL be
measured in pixels, because block gaps and card padding are not multiples of a
row. Frame cost MUST stay proportional to the number of visible rows and MUST NOT
grow with the size of the transcript; re-laying out the whole transcript on every
frame is not acceptable.

#### Scenario: Large transcript, steady frame

- **WHEN** the transcript holds tens of thousands of characters and a stream is appending to it
- **THEN** the per-frame layout work stays proportional to the visible rows

#### Scenario: Scrolling back during a stream

- **WHEN** the user scrolls up while an answer is streaming
- **THEN** the viewport stays where the user put it and does not snap back to the tail

#### Scenario: A line is one line

- **WHEN** the user presses `Down` once
- **THEN** the highlight moves exactly one text row, whether or not that row is inside a card

## ADDED Requirements

### Requirement: A reply is drawn in its own card

An answer, a failure and a note SHALL each be drawn inside a card: a rounded
rectangle of radius 12, inset `PAD` from the panel's left and right edges, filled
with a body tone distinct from the transcript's background, with 8 px of padding
above and below the text and the text inset a further 8 px, its corners
antialiased as the panel's are. A question SHALL NOT be a card: it SHALL stay one
row at `PAD` in `FG_PROMPT`, so the card is what marks the model's text. Blocks
SHALL be separated by an 8 px gap in place of a uniform row rhythm. A streaming
answer's card SHALL grow downwards with its text, and the padding, radius and
tone of every block SHALL NOT change while the stream is open.

#### Scenario: An answer is framed, a question is not

- **WHEN** a question and its finished answer are on screen
- **THEN** the answer's text sits inside a rounded card inset by `PAD`, and the question's row has no fill and starts at `PAD` like the bar's text

#### Scenario: The card follows the stream

- **WHEN** deltas keep arriving
- **THEN** the open card grows by each row its text adds, and every block above it keeps its position and tone

#### Scenario: A failure is a card, not list text

- **WHEN** the provider refuses the request
- **THEN** the failure is drawn inside its own card, distinct from the answer text, and the panel stays interactive

### Requirement: The highlighted line is marked by a rail

The highlighted transcript line SHALL be marked by a 2 px `BG_SEL` rail at the
left edge of its block's own inset, and MUST NOT be marked by a full-width band.
The rail SHALL lie inside the card when the highlighted line is in one, MUST NOT
change any block's width, height or position, and SHALL scroll with its line. The
highlight SHALL still name exactly one line for `Alt+C`.

#### Scenario: Moving the highlight moves only the rail

- **WHEN** the highlight moves from one line to the next
- **THEN** the two frames differ only in the rail's position, and no text moves

#### Scenario: Copying a line inside a card

- **WHEN** a line inside a card is highlighted and `Alt+C` is pressed
- **THEN** the clipboard holds that line's text and nothing else
