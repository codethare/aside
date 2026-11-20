## MODIFIED Requirements

### Requirement: The question stays in the bar while it is out

After `Return` the bar SHALL keep showing the question that was sent, for as long as
the request it started is unfinished. The line SHALL NOT be editable while it shows a
question: no caret and no pre-edit SHALL be drawn, and no key SHALL reach the text.

A question wider than the bar SHALL scroll from right to left at a slow, steady speed;
on reaching its end it SHALL hold, reverse, and hold again at its start, turning round
for as long as the request is unfinished. A question that fits SHALL NOT move.

The question SHALL leave the bar when the request ends — a complete answer, a
truncated answer, or a cancel — and when the reader writes into the line, which
starts a fresh question. A failed request instead hands the question back to the
input line, as it does today. When the request is what ends it, it SHALL NOT leave as
one sliding line: it SHALL first decompose, then be extracted.

The leaving SHALL freeze the state it was in at the instant the request ended: the
scroll offset and the direction the marquee was travelling are read once, when the
request ends, and every later frame uses those values. Over the first `DECOMPOSE`
milliseconds the glyphs SHALL fade out while particles at the glyphs' positions fade
in, so the line comes apart where it stands. Over the following `EXTRACT`
milliseconds those particles SHALL accelerate out of the edge the frozen direction
names — the right edge when the line was coming back, the left edge when it was
running out, and the left edge for a question that never moved — fading as they go.
The whole SHALL be over by `DECOMPOSE + EXTRACT` milliseconds, at which point the bar
shows the empty input line again.

Particles SHALL be seeded only from the visible glyphs, deterministically from their
index, with a fixed upper bound, and SHALL be clipped to the bar. The transcript SHALL
still draw no question, and the question SHALL NOT be written anywhere. Writing in the
bar during the leaving SHALL still drop the question at once.

#### Scenario: A long question is sent

- **WHEN** a question wider than the bar is sent and the model is still working
- **THEN** the bar still shows it, scrolling from its first character to its last and
  then back, and no caret is drawn

#### Scenario: The question decomposes and is extracted

- **WHEN** the request ends while the question is in the bar
- **THEN** the line first breaks into particles where it stands, and those particles
  are then drawn out of the edge it was heading for, the bar being an empty line once
  they are gone

#### Scenario: The exit direction is frozen

- **WHEN** the marquee would have turned round during the leaving
- **THEN** the particles still leave the side chosen at the instant the request ended,
  because the start offset and the direction were read once

#### Scenario: A short question is sent

- **WHEN** a question that fits in the bar is sent
- **THEN** it stays in the bar and does not move, and when the request ends it
  decomposes and leaves to the left

#### Scenario: The request ends

- **WHEN** the answer finishes, is truncated, or is cancelled
- **THEN** the bar is empty again after the leaving, showing no question and a caret
  if it has the keyboard

#### Scenario: A follow-up is begun while the answer streams

- **WHEN** the reader types into the bar while a request is still running
- **THEN** the question leaves the bar at once and the typing is what the line shows

#### Scenario: Copy mode takes the bar

- **WHEN** copy mode is entered while a question is still out
- **THEN** the bar shows the copy label, not the question, and shows the question again
  when copy mode is left
