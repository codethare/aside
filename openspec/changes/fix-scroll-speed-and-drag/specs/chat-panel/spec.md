## MODIFIED Requirements

### Requirement: The pointer scrolls the replies

While the pointer is over the replies card — above the bar, inside the panel — a
wheel notch or a two-finger touchpad slide SHALL scroll the transcript: a notch
SHALL move it five lines, a continuous delta SHALL move it by that many pixels, and
a scroll SHALL be clamped to the transcript's ends.

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

- **WHEN** the pointer is over the replies card and the wheel turns one notch
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

### Requirement: Reply lines can be selected with the pointer

A press of the left button on a reply line SHALL anchor a selection at the
**character** under the pointer; motion with the button down SHALL extend it to the
character under the pointer, releasing SHALL leave it in place, and pressing again
SHALL replace it. The selection SHALL be drawn as a band behind exactly the
characters it covers — the rows between its ends whole, the first and last cut where
the pointer named — with the text in the primary ink on top of it, and SHALL NOT
move any text. A press that is not on a reply line SHALL NOT start a selection, and
SHALL move the panel instead if it moves. A press in the scroll indicator's grab
column SHALL take hold of the indicator and start no selection, even though that
column lies over reply rows.

`Alt+C` and `Ctrl+C` SHALL copy the selected characters: the first and last lines
in part, the lines between whole, one line per line. `Esc` SHALL clear the selection before it cancels a stream or closes the
panel, sending a question SHALL clear it, and a selection MUST NOT survive a turn
change: the lines it named are no longer on screen.

#### Scenario: Selecting and copying three lines

- **WHEN** the user presses on the first line of an answer, drags to its third line, releases, and presses `Ctrl+C`
- **THEN** the clipboard holds those three lines, one per line, and nothing else

#### Scenario: Passing over the text selects nothing

- **WHEN** the pointer moves across the replies with no button held, after a selection has been made and released
- **THEN** the selection is unchanged, and no text is banded by the movement

#### Scenario: A selection can stop mid-word

- **WHEN** the user drags from the fourth word of a line to the second word of the next
- **THEN** only those characters are banded and copied, and the rest of each line is not

#### Scenario: Escape clears before it closes

- **WHEN** a selection exists and the user presses `Esc` with no stream open
- **THEN** the selection is cleared and the panel stays open; a second `Esc` closes it

#### Scenario: A selection cannot outlive its turn

- **WHEN** the user sends a question with a selection on screen
- **THEN** the selection is gone, and `Ctrl+C` copies the newly highlighted line only

## ADDED Requirements

### Requirement: The scroll indicator can be grabbed

While the transcript overflows the replies card, a press in the card's right
`PAD`-wide padding column — the column the 3 px strip is painted in, and where no
reply text is, because rows wrap at the same `PAD` — SHALL take hold of the
indicator. It SHALL start no text selection and no panel drag, SHALL leave an
existing selection alone as a wheel scroll does, and SHALL draw the indicator at full
ink for as long as it is held. A press in that column SHALL do nothing when the
transcript fits.

#### Scenario: The padding is not text

- **WHEN** the user presses in the card's right padding while the transcript overflows
- **THEN** the indicator is grabbed and no selection starts

#### Scenario: Held, the indicator stays

- **WHEN** the indicator is held with the button down for longer than its hold and
  its fade together
- **THEN** it is at full ink, because held is not idle

#### Scenario: Nothing to grab

- **WHEN** the pointer presses in the card's right padding and the transcript fits
  the card
- **THEN** nothing scrolls and the press is the padding's

### Requirement: A drag places the view where the thumb is

Motion with the button down SHALL place the view where the thumb is, in the mapping
the painter draws: the thumb's top travels the track's free span, so the track's top
SHALL show the transcript's first line and its bottom the last. A press outside the
thumb SHALL centre it under the pointer first. A drag to the bottom SHALL resume
following, and a drag away SHALL stop it.

#### Scenario: Dragging the indicator to the top

- **WHEN** the user presses on the indicator and drags it to the top of its track
- **THEN** the replies show the transcript's first line, and new text arriving no
  longer follows

#### Scenario: Dragging it to the bottom follows again

- **WHEN** the user drags the indicator to the bottom of its track
- **THEN** the replies show the newest line and follow the answer from then on

#### Scenario: A press on the track centres the thumb

- **WHEN** the user presses in the column well below the thumb
- **THEN** the thumb's middle moves to the pointer in that frame, and dragging from
  there carries the view
