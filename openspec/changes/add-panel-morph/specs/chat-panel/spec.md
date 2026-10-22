# Spec Delta

## ADDED Requirements

### Requirement: The panel rises into place

When the panel opens, its card SHALL grow up out of the screen's bottom edge over
200 ms on an ease-out, with its text and rim fading in behind it, and SHALL settle
into exactly the idle bar. The animation SHALL NOT change the surface's size, delay
a keystroke, or leave a frame at the end that differs from the settled one.

#### Scenario: Opening is not a snap

- **WHEN** the keybinding opens the panel
- **THEN** the bar is seen to rise out of the bottom edge, and its final frame is the idle bar pixel for pixel

#### Scenario: Typing during it still works

- **WHEN** a key arrives while the panel is still rising
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

### Requirement: The reveal is the replies card's own animation

The replies card appearing inside the answering panel SHALL keep its 240 ms reveal,
and SHALL start when the ball-to-panel morph ends rather than on top of it, so a
reply is seen once: first the card forming, then its content arriving.

#### Scenario: One motion at a time

- **WHEN** the first answer arrives
- **THEN** the ball grows into the panel, and only then does the replies card reveal
