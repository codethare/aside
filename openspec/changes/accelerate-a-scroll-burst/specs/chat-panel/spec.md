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

Scrolling up SHALL stop the view following the newest line; reaching the bottom
SHALL resume following. Scrolling over the bar SHALL do nothing. The scroll SHALL
NOT resize the panel, nor change which question the provider is sent.

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

- **WHEN** the pointer is over the input bar and the wheel turns
- **THEN** the transcript does not move

#### Scenario: Scrolling to the bottom follows again

- **WHEN** the view has been scrolled up and is then scrolled past the newest line
- **THEN** it stops at the newest line and follows new text from then on

## ADDED Requirements

### Requirement: A burst of scrolls accelerates

Scroll steps SHALL be counted as one burst while they arrive faster than `GAP`
(150 ms) apart. The burst's nth step SHALL move the view `1 + 0.5 (n - 1)` times the
distance a single step would have moved, and that multiplier MUST NOT exceed four.

A pause of 150 ms or more between wheel notches SHALL start a new burst, so the first
notch after any pause moves exactly five lines. A touchpad counts one step per
gesture — a run of axis events separated by less than 150 ms — so one slide SHALL
move the view by the distance slid, at whatever multiplier the burst had already
reached; a pause of 400 ms or more SHALL start a new burst for the touchpad too.

`Ctrl` with the wheel SHALL NOT advance nor apply a burst: it resizes the panel by
one size step per notch, however fast it is turned.

#### Scenario: A quick second notch travels farther

- **WHEN** the wheel is turned one notch, and again within 150 ms
- **THEN** the second notch moves the view half again as far as the first

#### Scenario: A spin grows to the cap and stops there

- **WHEN** the wheel keeps turning faster than 150 ms a notch
- **THEN** each notch moves farther than the one before, and no notch moves more than
  four times the five rows

#### Scenario: A pause returns a notch to five rows

- **WHEN** a burst is under way and the wheel is left alone for 150 ms or more, and
  then turned once
- **THEN** that notch moves five rows and the burst starts again

#### Scenario: One slide is one slide

- **WHEN** two fingers slide in one continuous gesture, however fast
- **THEN** the view moves by the distance the fingers moved, and no faster

#### Scenario: Quick slides accelerate

- **WHEN** two fingers make three quick slides, each starting within 400 ms of the
  last ending
- **THEN** the second slide moves farther than the first and the third farther than
  the second

#### Scenario: The size chord does not accelerate

- **WHEN** `Ctrl` and the wheel turn several notches quickly over the panel
- **THEN** the panel grows by one size step a notch and the scroll's burst is untouched
