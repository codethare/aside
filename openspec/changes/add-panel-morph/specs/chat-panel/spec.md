# Spec Delta

## ADDED Requirements

### Requirement: The panel opens as a line and closes back into one

When the panel opens it SHALL do so in two tracks over 520 ms: a line SHALL spread
out of a point at the centre, both ways at once, until it has the panel's width, and
then the card SHALL unfold upward out of that line into exactly the idle bar, with
its text and rim fading in behind it. Only a point is on screen at the start, and a
line — not a card — before the unfold begins. The animation SHALL NOT change the
surface's size, delay a keystroke, or leave a frame at the end that differs from the
settled one.

Closing SHALL be the same run reversed, over 460 ms: the card rolls up into a line,
and then that line pulls in from both ends to a point and is gone. `Esc` in typing
mode with nothing to cancel therefore closes the panel **and then exits**, rather
than exiting in one frame; no key SHALL have a meaning while the closing runs.

#### Scenario: Opening is not a snap

- **WHEN** the keybinding opens the panel
- **THEN** a line is seen to spread out of the centre and unfold into the bar, and its final frame is the idle bar pixel for pixel

#### Scenario: Closing is the opening backwards

- **WHEN** the user presses `Esc` with no stream open
- **THEN** the bar rolls up into a line, the line pulls in to the centre, and the process exits after it

#### Scenario: Typing during it still works

- **WHEN** a key arrives while the panel is still opening
- **THEN** it reaches the input line and the animation is not restarted

### Requirement: The bar morphs into the ball and back out

Sending a question SHALL morph the bar's card into the thinking ball over 320 ms on
a spring: one rounded card whose width, height and corner radius interpolate from
the bar's band to the 60 px square circle, the bar's text fading out and the ball's
squares fading in. The first text SHALL morph that card out into the answering
panel over 400 ms, the replies fading up inside it and the bar's own line fading in
at the end. Both transitions SHALL be painted inside one buffer at the larger of
the two states' sizes, so the surface SHALL be resized once per transition and
`MUST NOT` be resized per frame. A settled frame SHALL be painted by the static
painters exactly as before, and neither transition SHALL re-wrap the transcript.

#### Scenario: The bar becomes the ball

- **WHEN** `Enter` sends a question
- **THEN** the bar's card is seen to shrink into the circle, and the first frame at rest is the thinking ball

#### Scenario: The ball becomes the panel

- **WHEN** the first text of an answer arrives
- **THEN** the circle grows into the panel, the replies fade up inside it, and the first frame at rest is the answering panel

#### Scenario: No resize storm

- **WHEN** either transition runs
- **THEN** the surface is given one size and commits no size change until the transition ends

#### Scenario: Escape mid-morph

- **WHEN** the user cancels while the ball is morphing out
- **THEN** the panel settles at the answering size with the cancelled note, without waiting for the morph

## MODIFIED Requirements

### Requirement: The panel enters once

The replies SHALL enter with the panel's own growth and SHALL NOT be animated again
after it: once the panel has become the answering panel, the replies card fills it
in that frame. A second animation inside the finished panel reads as the panel
opening twice, which is the defect this rule exists to prevent.

#### Scenario: One motion, not two

- **WHEN** the first answer arrives
- **THEN** the panel grows into the answering panel and the reply is on screen when it arrives, with nothing animating after it
