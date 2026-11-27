# Spec Delta

## ADDED Requirements

### Requirement: The cat stands at a modern angle

The cat's head SHALL sit at `(0.65w, 0.34h)` with the same radius, and SHALL rest
with a tilt of about 6 degrees toward the panel. The ears SHALL turn about the
head's centre, staying on the head ring. The face SHALL turn about the mouth, so
the mouth stays the fixed point the bubble's tail and growth origin use. The body
SHALL be taller and asymmetric than a symmetric loaf, and the tail SHALL carry a
small loop.

#### Scenario: The tilt does not move the mouth

- **WHEN** the cat is waiting at any stage size
- **THEN** the mouth is the point `mouth_at` returns, and the bubble's tail and
  growth origin are unchanged by the tilt

#### Scenario: The paw contact survives the stance

- **WHEN** the cat has landed at any buffer scale
- **THEN** the paw mitten is still tangent to the stage's right edge at the input
  card's vertical centre

### Requirement: Poses are spring-eased and varied

The paw's reach SHALL use an ease-out-back curve that overshoots the contact by no
more than 3 % and settles at exactly the contact. The hop's horizontal travel
SHALL be a smoothstep. The lick SHALL flick and pause on a 620 ms cycle with a
head bob and a tongue sway. The speaking mouth SHALL open in two parts, not one
bump. The tail SHALL sway and an ear SHALL twitch during the active poses, at
different frequencies.

#### Scenario: The reach settles on the pad

- **WHEN** the arm or the answer-end reach is complete
- **THEN** the mitten's right edge is exactly the card's left rim, with no
  residual overshoot

#### Scenario: The lick is not a metronome

- **WHEN** the cat licks while the request runs
- **THEN** the tongue moves in flicks with pauses, and the head, tail and ears
  move on different rhythms

#### Scenario: The speaking mouth has two beats

- **WHEN** the answer's first text arrives
- **THEN** the mouth opens, eases, and opens once more before it closes

### Requirement: The waiting cat blinks

While the cat is waiting, the eyes SHALL close for about 260 ms every 5 s, drawn
as short horizontal strokes. The stage SHALL ask for a frame while waiting so the
blink is painted, without a continuous animation loop.

#### Scenario: The waiting cat blinks

- **WHEN** the cat has been waiting for five seconds
- **THEN** one frame shows the eyes closed, and the next shows the dots again

#### Scenario: The blink is not a loop

- **WHEN** the cat is waiting with no request and no motion
- **THEN** the stage is repainted at the idle tick, not every 16 ms
