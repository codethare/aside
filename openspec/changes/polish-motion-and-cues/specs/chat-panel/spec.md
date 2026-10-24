# Spec Delta

## MODIFIED Requirements

### Requirement: The panel changes shape without overshooting

A change of shape — the bar growing into the replies, and any later change of the
same kind — SHALL be drawn as an ease-out over its whole duration, 360 ms, or 460 ms
when the bar grows into the replies, interpolating the card's position, width, height
and corner radius between the shape being left and the shape arriving, the content of
each fading in its turn. The change SHALL be monotone from the one shape to the other,
SHALL reach the arriving shape exactly at its deadline, and MUST NOT pass it: no frame
may show a card larger than the one the change lands on, and no frame before a third
of the duration may be the settled one. Both transitions SHALL be painted inside one
buffer at the larger of the two shapes, so the surface SHALL be resized once per
transition and MUST NOT be resized per frame. A settled frame SHALL be painted by the
static painters exactly as before, and neither transition SHALL re-wrap the
transcript.

#### Scenario: The change is over when the deadline says

- **WHEN** the frames of a shape change are taken a third of the way through and at its deadline
- **THEN** the early frame is visibly part-way between the two shapes, and the frame at the deadline is the arriving shape pixel for pixel

#### Scenario: Nothing is painted larger than it lands

- **WHEN** any frame of a shape change is measured against the shape it is settling into
- **THEN** no width, height or corner radius exceeds the settled one

#### Scenario: No resize storm

- **WHEN** either transition runs
- **THEN** the surface is given one size and commits no size change until the transition ends

### Requirement: The working light travels the bar's rim

Once a request has been open for longer than 500 ms the panel SHALL show a light
running around the bar card's rounded outline: a bright segment, a share of the
perimeter long, whose head advances one lap every 1.4 s, drawn over the rim's own lit
and dark profile and fading out behind itself rather than stopping at an end. The
light MUST NOT appear before that threshold, so a request answered sooner SHALL never
show it, and its lap SHALL be timed from the moment the light appears rather than
from the moment the request was sent. It SHALL occupy no space, change no geometry,
and be gone when the stream ends, with nothing on screen shifting. The light's
position SHALL come from the clock rather than from a frame count, so a late frame
lands where the time says.

#### Scenario: A fast answer never shows it

- **WHEN** the first text of an answer arrives 300 ms after `Enter`
- **THEN** no frame of that request shows the light

#### Scenario: The light waits, then starts at the rim's start

- **WHEN** frames are painted 200 ms and 600 ms after `Enter`
- **THEN** the bar carries no light at 200 ms, and at 600 ms the light is a short way past the start of the rim rather than a third of a lap round it

#### Scenario: The light goes round

- **WHEN** two frames 700 ms apart are painted while a request is open
- **THEN** the lit part of the rim is on the opposite side of the card

#### Scenario: A late frame lands on the right place

- **WHEN** the loop wakes late, so several laps' worth of time has passed
- **THEN** the light is where the elapsed time puts it, not where a frame count would

#### Scenario: Indicator disappears without a jump

- **WHEN** a stream ends
- **THEN** the bar's size, its text and every reply line are where they were, and only the light is gone

### Requirement: The panel opens as a line and closes back into one

When the panel opens it SHALL do so in two tracks over 520 ms: a line SHALL spread
out of a point at the centre, both ways at once, until it has the panel's width, and
then the card SHALL unfold upward out of that line into exactly the idle bar, with
its text and rim fading in behind it. Only a point is on screen at the start, and a
line — not a card — before the unfold begins. The animation SHALL NOT change the
surface's size, delay a keystroke, or leave a frame at the end that differs from the
settled one.

Closing SHALL be the same run reversed, over 200 ms — quicker than the opening, so
that an exit is never as slow as an arrival: the card rolls up into a line, and then
that line pulls in from both ends to a point and is gone. `Esc` in typing mode with
nothing to cancel therefore closes the panel **and then exits**, rather than exiting
in one frame; no key SHALL have a meaning while the closing runs.

#### Scenario: Opening is not a snap

- **WHEN** the keybinding opens the panel
- **THEN** a line is seen to spread out of the centre and unfold into the bar, and its final frame is the idle bar pixel for pixel

#### Scenario: Closing is the opening backwards, and shorter

- **WHEN** the user presses `Esc` with no stream open
- **THEN** the bar rolls up into a line, the line pulls in to the centre, and the process has exited within 200 ms of the key

#### Scenario: Typing during it still works

- **WHEN** a key arrives while the panel is still opening
- **THEN** it reaches the input line and the animation is not restarted

## ADDED Requirements

### Requirement: The scroll indicator shows only while the view moves

The panel SHALL draw its 3 px indicator on the replies' right edge only while the
view has recently been moved, and MUST NOT show it at rest. A scroll the user makes —
a wheel notch, a touchpad slide, or a page key — SHALL raise it at full ink for
700 ms, after which it SHALL fade to nothing over 200 ms and not be drawn. Text
arriving at the tail SHALL NOT raise it, and it MUST NOT be drawn when the transcript
fits the card. It SHALL occupy no space, change no geometry, and MUST NOT move any
row or the text column.

#### Scenario: A scroll shows it, then it goes

- **WHEN** the user scrolls once and then leaves the view alone
- **THEN** the indicator is at full ink, part-way through its fade 800 ms later, and gone at 900 ms

#### Scenario: An answer does not raise it

- **WHEN** text streams into a transcript that overflows the card and the user has not scrolled
- **THEN** no indicator is drawn

#### Scenario: Nothing shifts

- **WHEN** the indicator appears and fades away
- **THEN** every row and the card's size are exactly where they were

### Requirement: The replies dissolve at the card's bottom edge

The replies SHALL fade their rows out over the last row of the card rather than
cutting them at the edge: a row at the edge SHALL be drawn at nothing, a row one row
above it SHALL be drawn whole, and the factor SHALL be linear in between. The ramp
SHALL be one row tall, so that a row that fits inside the card is never dimmed, and
it SHALL follow the font size and the output scale with the rows themselves. The
same factor SHALL apply to a selection band and to the copy cursor, so that a
selection reaching the edge does not leave a hard rectangle behind faded glyphs.
Rows further than that from the edge SHALL be drawn at their own full strength, and
the dissolve MUST NOT change any row's position, the card's size, the title and
body inks of rows above the ramp, or the frame cost — one factor per visible row.
Only the bottom edge dissolves; a row partly scrolled out above the card's top is
still not drawn at all.

#### Scenario: A line leaves instead of ending

- **WHEN** a row sits inside the card's last row of height
- **THEN** its ink is dimmer than the same row above the ramp, and dimmest at the edge

#### Scenario: Rows that fit are untouched

- **WHEN** every visible row lies a whole row above the card's bottom edge
- **THEN** every row is drawn at its own full strength and nothing is dimmed

#### Scenario: A selection at the edge fades with its text

- **WHEN** a pointer's selection covers a row inside the ramp
- **THEN** its band is dimmed by the same factor as the characters on it
