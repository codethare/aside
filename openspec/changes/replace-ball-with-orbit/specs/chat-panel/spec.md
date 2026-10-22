# Spec Delta

## MODIFIED Requirements

### Requirement: The panel is a layer surface in two sizes

The panel SHALL be a `wlr-layer-shell` surface on the overlay layer, anchored to the
bottom and left edges, horizontally centred by its own left margin, with exclusive
keyboard interactivity. Its bottom edge SHALL sit 38 % of the output height above
the screen's bottom edge. It SHALL have exactly three states and **two** sizes:

| State | Panel size (logical px, 1×) | Content |
|---|---|---|
| idle | 640 × 30 | the input bar alone |
| thinking | 640 × 30 | the same bar, with a light travelling its rim |
| answering | 640 × (bar + 16 + cap), fixed | the replies above, the bar below |

Sending a question SHALL NOT change the panel's size or shape: the state changes,
the geometry does not. The surface carries the 20 px shadow margin inside its own
size.

#### Scenario: Summoned idle

- **WHEN** the panel opens with nothing asked
- **THEN** only the bar is on screen, and its bottom edge is 38 % of the output height above the screen's bottom edge

#### Scenario: Enter lights the rim

- **WHEN** the user presses `Enter` with a non-empty input
- **THEN** the request starts, the panel keeps its size, and a light begins travelling the bar's rim

#### Scenario: The first delta grows the panel

- **WHEN** the answer's first text arrives
- **THEN** the bar morphs into the answering panel to the ball's absence, the light stops, and the replies fade up

#### Scenario: Typing during the light is ignored

- **WHEN** the panel is thinking and a printable key is pressed
- **THEN** the input is unchanged, and `Esc` still cancels the request

### Requirement: The working light travels the bar's rim

While a request is open the panel SHALL show a light running around the bar card's
rounded outline: a bright segment, a share of the perimeter long, whose head
advances one lap every 1.4 s, drawn over the rim's own lit and dark profile and
fading out behind itself rather than stopping at an end. It SHALL occupy no space,
change no geometry, and be gone when the stream ends, with nothing on screen
shifting. The light's position SHALL come from the clock rather than from a frame
count, so a late frame lands where the time says.

#### Scenario: The light goes round

- **WHEN** two frames 700 ms apart are painted while a request is open
- **THEN** the lit part of the rim is on the opposite side of the card

#### Scenario: A late frame lands on the right place

- **WHEN** the loop wakes late, so several laps' worth of time has passed
- **THEN** the light is where the elapsed time puts it, not where a frame count would

#### Scenario: Indicator disappears without a jump

- **WHEN** a stream ends
- **THEN** the bar's size, its text and every reply line are where they were, and only the light is gone
