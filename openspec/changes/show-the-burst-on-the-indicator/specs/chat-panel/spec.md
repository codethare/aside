## MODIFIED Requirements

### Requirement: The scroll indicator shows only while the view moves

The panel SHALL draw its 3 px indicator on the replies' right edge only while the
view has recently been moved, and MUST NOT show it at rest. A scroll the user makes —
a wheel notch, a touchpad slide, or a page key — SHALL raise it at full ink for
700 ms, after which it SHALL fade to nothing over 200 ms and not be drawn. Text
arriving at the tail SHALL NOT raise it, and it MUST NOT be drawn when the transcript
fits the card. It SHALL occupy no space, change no geometry, and MUST NOT move any
row or the text column.

While a burst of scrolls is accelerating, the indicator SHALL also carry the burst's
multiplier, set on the strip (see the multiplier requirement below).

#### Scenario: A scroll shows it, then it goes

- **WHEN** the user scrolls once and then leaves the view alone
- **THEN** the indicator is at full ink, part-way through its fade 800 ms later, and gone at 900 ms

#### Scenario: An answer does not raise it

- **WHEN** text streams into a transcript that overflows the card and the user has not scrolled
- **THEN** no indicator is drawn

#### Scenario: Nothing shifts

- **WHEN** the indicator appears and fades away
- **THEN** every row and the card's size are exactly where they were

## ADDED Requirements

### Requirement: The strip carries the burst's multiplier

While a burst is accelerating — its multiplier above one — the indicator SHALL show
that multiplier as text in the strip's own column, centred on the strip's width and on
the thumb's middle, so the number belongs to the indicator rather than to a widget
beside it. The strip SHALL NOT change shape: it stays 3 px wide and its length keeps
showing the view's position, and the number SHALL be drawn over it.

The number SHALL be `1.5x`, `2x`, `3.5x`, `4x` — one decimal only when the multiplier
is not whole — and SHALL be set in a face smaller than the panel's own text, so a
number on the right edge cannot be read as a line of the answer. It SHALL be drawn in
the panel's text ink at the indicator's own strength, so it fades with the strip, and
it SHALL be clamped inside the replies card, never past its right edge nor above or
below its rows.

The number SHALL be drawn exactly while the multiplier stands above one: on the first
frame after the burst's own window (400 ms) has passed without a step it SHALL be
gone, and the strip SHALL carry on fading on its own clock. A single touchpad slide
SHALL show nothing, because one gesture is one step and its multiplier is one.

#### Scenario: A burst writes the multiplier on the strip

- **WHEN** the wheel turns fast enough for the burst to reach twice a step
- **THEN** the strip is 3 px wide as it always was, with `2x` set over it at the thumb's middle

#### Scenario: The number grows to the cap and stops

- **WHEN** the burst runs on past four times
- **THEN** the number reads `4x` and no more

#### Scenario: The burst ends and the number goes

- **WHEN** the wheel is left alone for 400 ms while the strip still has ink
- **THEN** no number is drawn, and the plain strip fades on its own clock

#### Scenario: A slow scroll shows no number

- **WHEN** the wheel turns a single notch, or two fingers slide once
- **THEN** the indicator is the 3 px strip and no number is drawn

#### Scenario: The number stays on the card

- **WHEN** the thumb is at either end of its track and the number is drawn
- **THEN** it is inside the replies card's right edge and inside its rows, and no row moves

#### Scenario: The number is smaller than the answer

- **WHEN** the number is drawn beside the answer's own text
- **THEN** its glyphs are shorter than a row of the panel's text, so the answer's lines read as they did
