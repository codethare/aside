## MODIFIED Requirements

### Requirement: The pointer scrolls the replies

While the pointer is over the replies card — above the bar, inside the panel — a
wheel notch or a two-finger touchpad slide SHALL scroll the transcript: a notch
SHALL move it five lines, a continuous delta SHALL move it by that many pixels, and
a scroll SHALL be clamped to the transcript's ends. A notch that is the first of a
burst SHALL move exactly five lines; a notch that repeats a burst SHALL move farther
(see the burst requirement below).

A wheel's distance SHALL be read from the high-resolution step count when the
compositor sends one (120 units is a notch), from the deprecated discrete count
after that, and from the axis distance last — and the axis distance SHALL be read as
notches only when the axis source is the wheel, because on a touchpad it is already
pixels. A wheel notch MUST NOT be one axis unit.

No part of a scroll SHALL be dropped: a distance smaller than one pixel SHALL be
carried into the next event rather than rounded away, so a slide that sends tenths
of a pixel at a time moves the view at the rate the fingers move.

The scrolling region SHALL be the replies card and nothing else: it SHALL end at the
gap that separates the card from the bar, and scrolling over that gap, over the bar,
or over the panel's shadow margin SHALL do nothing.

Scrolling up SHALL stop the view following the newest line; reaching the bottom
SHALL resume following. The scroll SHALL NOT resize the panel, nor change which
question the provider is sent.

#### Scenario: A wheel notch over the replies moves five lines

- **WHEN** the pointer is over the replies card and the wheel turns one notch, alone
- **THEN** the view moves five lines toward the direction of the turn

#### Scenario: A notch is counted from whichever field carries it

- **WHEN** a wheel notch arrives as 120 high-resolution units, and another arrives
  as one deprecated discrete step, and another as 15 axis units from a wheel
- **THEN** each of the three moves the view the same five lines

#### Scenario: A touchpad slide scrolls

- **WHEN** two fingers slide up over the replies card
- **THEN** the transcript scrolls by the distance slid, without a notch to quantize it

#### Scenario: A slide too small for a pixel is not lost

- **WHEN** two fingers slide in events that each ask for less than one pixel
- **THEN** the whole pixels they add up to are scrolled, and the remainder is kept
  for the next event

#### Scenario: Over the bar, nothing scrolls

- **WHEN** the pointer is over the input bar, at its top row or its last, and the
  wheel turns
- **THEN** the transcript does not move, and an answer that is streaming stays at
  its newest line

#### Scenario: The gap between the cards is not the replies

- **WHEN** the pointer is in the 16 px between the replies card and the bar and the
  wheel turns
- **THEN** the transcript does not move

#### Scenario: Scrolling to the bottom follows again

- **WHEN** the view has been scrolled up and is then scrolled past the newest line
- **THEN** it stops at the newest line and follows new text from then on
