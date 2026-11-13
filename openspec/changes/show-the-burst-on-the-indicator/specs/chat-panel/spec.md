## MODIFIED Requirements

### Requirement: The scroll indicator shows only while the view moves

The panel SHALL draw its 3 px indicator on the replies' right edge only while the
view has recently been moved, and MUST NOT show it at rest. A scroll the user makes —
a wheel notch, a touchpad slide, or a page key — SHALL raise it at full ink for
700 ms, after which it SHALL fade to nothing over 200 ms and not be drawn. Text
arriving at the tail SHALL NOT raise it, and it MUST NOT be drawn when the transcript
fits the card. It SHALL occupy no space, change no geometry, and MUST NOT move any
row or the text column.

While a burst of scrolls is accelerating, the indicator SHALL be a capsule in the
strip's place rather than a 3 px strip (see the burst requirement below).

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

### Requirement: The indicator becomes a capsule with the burst's multiplier

While a burst is accelerating — its multiplier above one — the indicator SHALL be a
capsule: a pill filled with the indicator's own accent ink, one radius of half its
height, drawn in the thumb's place, at least as tall as the panel's row, right-aligned
inside the replies card and clamped to its rows so it never leaves the card.

The capsule SHALL show the burst's multiplier as text in the panel's normal ink — `1.5x`,
`2x`, `3.5x`, `4x`, one decimal only when the multiplier is not whole — and SHALL be
no narrower than that text plus its own padding, so the number is never cut.

The capsule SHALL be drawn exactly while the multiplier stands above one: it SHALL go
back to being the 3 px strip on the first frame after the burst's own window (400 ms)
has passed without a step, and SHALL NOT be drawn at all when the view has settled and
the indicator's ink has faded. A single touchpad slide SHALL leave the strip as it is,
because one gesture is one step and its multiplier is one.

#### Scenario: A burst turns the strip into a capsule

- **WHEN** the wheel turns fast enough for the burst to reach twice a step
- **THEN** the indicator is a capsule reading `2x`, at the thumb's place on the card's right edge

#### Scenario: The capsule grows to the cap and stops

- **WHEN** the burst runs on past four times
- **THEN** the capsule reads `4x` and no more

#### Scenario: The burst ends and the strip comes back

- **WHEN** the wheel is left alone for 400 ms while the view still has ink on its indicator
- **THEN** the indicator is the plain 3 px strip again, at the thumb's place

#### Scenario: A slow scroll is still a strip

- **WHEN** the wheel turns a single notch, or two fingers slide once
- **THEN** the indicator is the 3 px strip and no number is drawn

#### Scenario: The capsule stays on the card

- **WHEN** the thumb is at either end of its track and the capsule is drawn
- **THEN** the capsule is inside the replies card's rows and inside its right edge
