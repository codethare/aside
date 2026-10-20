# Spec Delta

## MODIFIED Requirements

### Requirement: The panel is a layer surface in three states

The panel SHALL be a `wlr-layer-shell` surface on the overlay layer, anchored to
the **bottom** edge with a 32 px margin, horizontally centred on its output, with
exclusive keyboard interactivity. The bottom anchor is what keeps the bar in one
place on screen while the answer grows: a top anchor would carry it downwards by
a row per wrapped line. It SHALL have exactly three states:

| State | Panel size (logical px, 1×) | Content |
|---|---|---|
| idle | 640 × 30 | the input bar alone |
| thinking | 60 × 60 | a filled circle with four blinking squares |
| answering | 640 × (bar_h + divider + transcript height), capped | the replies above, the bar below |

`row_h` is `row_h_for(20)` (30 at 1×). The transcript height is the sum of its
blocks' heights — each block being one text row, or a card's text rows plus its
padding — together with the gap between blocks, and it SHALL NOT exceed the
height of 64 text rows. A size change SHALL be committed before the frame that
uses it. These are the panel's sizes: the surface carries the frame margin below,
so its size is `panel + 2 × 24` on each axis and its bottom margin is 8 px — the
panel keeps a 32 px visible gap to the screen edge.

#### Scenario: Summoned idle

- **WHEN** the panel opens with nothing asked
- **THEN** only the bar is on screen: a 640 × 30 panel, no transcript area, inside a 688 × 78 surface

#### Scenario: Enter starts the ball

- **WHEN** the user presses `Enter` with a non-empty input
- **THEN** the request starts and the panel becomes a 60 × 60 square centred on the bar, painting a circle

#### Scenario: The first delta expands the panel upwards

- **WHEN** the answer's first text arrives
- **THEN** the panel returns to 640 wide, grows upwards out of the bar, and the bar stays at the height it held while idle

#### Scenario: Typing during the ball is ignored

- **WHEN** the panel is thinking and a printable key is pressed
- **THEN** the input is unchanged, and `Esc` still cancels the request

### Requirement: The bar matches wlmenu's bar

The bar SHALL use `wlmenu`'s metrics unchanged: width 640, text size 20 px,
row height `row_h_for(20)`, `PAD` 16, corner radius 12, an 8 px gap after a 4 px
`LABEL_ACCENT` rail, and a 1 px divider at 25 % alpha separating it from the
transcript. The bar SHALL be the **bottom** row of the answering panel, below the
replies, with the divider along its top edge. While idle the bar is the whole
panel, so the two placements are the same surface.

#### Scenario: One bar, two products

- **WHEN** the idle panel and `wlmenu`'s bar are compared at the same scale
- **THEN** their widths, heights, padding, radius, rail and divider are identical

#### Scenario: The bar holds its place

- **WHEN** an answer wraps to more rows and the panel grows
- **THEN** the bar's pixels on screen are the same before and after, and only the area above it changed

### Requirement: The transcript draws only the replies

The transcript SHALL show what the model or the panel said — answers, failures
and notes — and SHALL NOT show the questions the user typed. A question SHALL
still be kept in memory for the request that follows it, and SHALL take no rows
and no height on screen. `Up`, `Down`, `PageUp`, `PageDown` and `Alt+C` SHALL
address reply lines alone, and the copy key SHALL never carry a question the user
already sent.

#### Scenario: Only the reply is on screen

- **WHEN** a finished question and its answer are on screen
- **THEN** the answer's card is there and the question appears nowhere in the panel

#### Scenario: The follow-up still carries the history

- **WHEN** the user asks a second question whose own row is not drawn
- **THEN** the request carries the first question and its answer, in order, as before

### Requirement: Turns accumulate in one transcript

The panel SHALL keep every turn of the current run in one ordered transcript, and
sending a question MUST keep all earlier replies on screen. Each request MUST
carry the preceding turns of this run in order, and MUST NOT carry anything from
a previous run of the process. An answer that was cancelled or cut short SHALL stay
visible in the transcript, and MUST be left out of subsequent requests rather than
sent back to the provider as if it were complete.

#### Scenario: A follow-up keeps the first answer

- **WHEN** the user asks a question, receives an answer, and asks a follow-up
- **THEN** the first answer stays on screen, and the request body contains the first question, the first answer and the follow-up

#### Scenario: Nothing survives a restart

- **WHEN** `aside` is closed and started again
- **THEN** the transcript is empty and the request contains only the new question
