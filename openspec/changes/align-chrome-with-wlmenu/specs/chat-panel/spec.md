# Spec Delta

## MODIFIED Requirements

### Requirement: The panel is a layer surface in three states

The panel SHALL be a `wlr-layer-shell` surface on the overlay layer, anchored to
the **bottom** edge, horizontally centred on its output, with exclusive keyboard
interactivity. Its bottom edge SHALL sit **38 % of the output height** above the
screen's bottom edge, so the input field lands just below the vertical centre and
the replies above it stay where the eye already is. The bottom anchor is what
keeps the field still while the replies grow. It SHALL have exactly three states:

| State | Panel size (logical px, 1×) | Content |
|---|---|---|
| idle | 640 × 30 | the input bar alone |
| thinking | 60 × 60 | a filled circle with four blinking squares |
| answering | 640 × (bar_h + 12 + replies), capped | the replies above, the bar below, 12 px apart |

`row_h` is `row_h_for(20)` (30 at 1×). The replies height is the sum of the reply
blocks' rows together with the gap between them, and SHALL NOT exceed the height
of 64 text rows. The cap on the panel is `output_height − bottom margin − 32 px`
of breathing room above it. A size change SHALL be committed before the frame that
uses it; the surface carries the 20 px shadow margin inside its own size.

#### Scenario: Summoned idle

- **WHEN** the panel opens with nothing asked
- **THEN** only the bar is on screen, and its bottom edge is 38 % of the output height above the screen's bottom edge

#### Scenario: Enter starts the ball

- **WHEN** the user presses `Enter` with a non-empty input
- **THEN** the request starts and the panel becomes a 60 × 60 square centred on the bar, painting a circle

#### Scenario: The first delta expands the panel upwards

- **WHEN** the answer's first text arrives
- **THEN** the panel returns to 640 wide, grows upwards out of the bar, and the bar stays where it was

#### Scenario: A tall answer never crosses the top edge

- **WHEN** the replies are taller than the cap
- **THEN** the panel stops at `output_height − bottom margin − 32` and the replies scroll inside it

### Requirement: The bar matches wlmenu's bar

The bar SHALL use `wlmenu`'s metrics unchanged: width 640, text size 20 px, row
height `row_h_for(20)`, `PAD` 16, corner radius 12, and an 8 px gap after a 4 px
`LABEL_ACCENT` rail where `wlmenu` draws one — which is only when it is given a
prompt label. `aside` has no label, so its bar SHALL carry **no rail**. The bar
SHALL be the bottom card of the answering panel, 12 px below the replies card, and
while idle it is the whole panel.

#### Scenario: One bar, two products

- **WHEN** the idle panel and `wlmenu`'s bar are compared at the same scale
- **THEN** their widths, heights, padding and radius are identical, and neither draws a rail, because neither is given a label

#### Scenario: The bar holds its place

- **WHEN** an answer wraps to more rows and the panel grows
- **THEN** the bar's pixels on screen are the same before and after, and only the area above it changed

### Requirement: The frame matches wlmenu's frame

The two cards SHALL be drawn as `wlmenu` draws its own: a premultiplied
translucent body, a 20 px shadow margin holding both drop-shadow layers —
`(blur 6, alpha 155, offset 1)` and `(blur 13, alpha 115, offset 4)` — measured
from each card's rounded outline so the falloff wraps the corners, a one-pixel
hairline using the lit and dark profiles, antialiased corners, and the 3 px
`LABEL_ACCENT` scroll indicator at the right edge while the replies overflow. The
bar and the replies SHALL share one body tone, `BG_PROMPT`, and the 12 px between
them SHALL show nothing but their own shadows. The thinking ball SHALL use the
same body, border and shadow.

#### Scenario: One panel, two products

- **WHEN** a screenshot of `aside`'s answering panel and one of `wlmenu` are laid side by side
- **THEN** their shadow falloff, body translucency, hairline, corner shape and card separation agree

#### Scenario: One grey

- **WHEN** the answering panel is inspected row by row
- **THEN** the bar card and the replies card are the same tone, and no reply block is filled differently from the card it sits in

### Requirement: The transcript draws only the replies

The transcript SHALL show what the model or the panel said — answers, failures
and notes — and SHALL NOT show the questions the user typed. A question SHALL
still be kept in memory for the request that follows it, and SHALL take no rows
and no height on screen. Two replies SHALL be separated by an 8 px gap and by
nothing else: no fill, no border, no rule between them. `Up`, `Down`, `PageUp`,
`PageDown` and `Alt+C` SHALL address reply lines alone.

#### Scenario: Only the reply is on screen

- **WHEN** a finished question and its answer are on screen
- **THEN** the answer's lines are there and the question appears nowhere in the panel

#### Scenario: The follow-up still carries the history

- **WHEN** the user asks a second question whose own row is not drawn
- **THEN** the request carries the first question and its answer, in order, as before

### Requirement: The highlighted line is marked by ink

The highlighted reply line SHALL be marked by drawing its text in `FG_SEL` and
shifting it 2 px in from the line above and below. `aside` SHALL NOT draw a rail,
a band or any other mark in the margin for the highlight. The mark SHALL NOT
change any block's height or the panel's size, SHALL scroll with its line, and
the highlight SHALL still name exactly one line for `Alt+C`.

#### Scenario: Moving the highlight moves ink, not geometry

- **WHEN** the highlight moves from one line to the next
- **THEN** the panel's size and every line's vertical position are unchanged, and only which line is brighter differs

#### Scenario: Copying a line

- **WHEN** a highlighted reply line is copied with `Alt+C`
- **THEN** the clipboard holds that line's text and nothing else

### Requirement: The working indicator never moves the layout

While a request is open the panel SHALL show that it is working without occupying
space or changing any geometry. Before the first text the thinking ball SHALL be
that indication; once text is arriving, the reply appearing and growing SHALL be
the indication, and the bar SHALL carry no mark of its own - `wlmenu` has no such
mark, and a rail in the field was the last of the old chrome. The indication MUST
be gone when the stream ends, with nothing on screen shifting.

#### Scenario: Indicator disappears without a jump

- **WHEN** a stream ends
- **THEN** the rendered position of every reply line is unchanged from the frame before the stream ended

#### Scenario: Working, with nothing extra on screen

- **WHEN** the ball is on screen and no text has arrived
- **THEN** the ball is the only indication, and when text arrives the bar looks exactly as it does when idle

## ADDED Requirements

### Requirement: The replies card pops out of the bar

The first time the replies card appears in a run, it SHALL grow upward out of the
bar over 160 ms with an ease-out, its body, hairline, shadow and text clipped to
the height it has reached, so a reply is visibly produced by the field rather than
flashing into place. The animation SHALL NOT re-wrap text, change any key, move
the bar, or delay a keystroke; a frame SHALL be painted while it runs. A later
answer in the same run SHALL NOT replay it, and the panel SHALL NOT animate at all
when it opens, closes or shows the ball.

#### Scenario: A reply is produced by the field

- **WHEN** the first text of an answer arrives
- **THEN** the replies card is seen to rise out of the bar, and the bar itself does not move

#### Scenario: A follow-up does not replay it

- **WHEN** the second answer of a run arrives
- **THEN** its card is on screen whole, once, with no reveal

## REMOVED Requirements

### Requirement: The highlighted line is marked by a rail

**Reason**: `wlmenu`'s current chrome has no rail without a prompt label, and a
blue line in the margin was the last piece of the old list look.

**Migration**: the highlight is now brighter ink, 2 px in from its neighbours; the
key table, `Alt+C` and the scroll behaviour are unchanged.
