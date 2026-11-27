# Spec Delta

## ADDED Requirements

### Requirement: The cat is filled with paper and outlined in ink

The stage SHALL fill the cat's body, head, ears, arm, tail and paws with a flat
paper colour, and SHALL outline every fill with a warm near-black ink line in three
roles: silhouette about 1.15 % of the stage's side, part about 0.8 %, detail about
0.55 %.

The body SHALL be one filled Catmull-Rom contour closed under the head; the tail
and arm SHALL be paper strokes inside an ink border. Layers SHALL be drawn back to
front: tail, arm, body, ears, head, paws, face.

#### Scenario: The body is filled, not a wireframe

- **WHEN** the cat is in the waiting pose
- **THEN** the body interior and the head centre are paper, and the body's outline
  and the head's ring are ink

#### Scenario: Every fill has an edge

- **WHEN** the cat is drawn at any stage size
- **THEN** the body, head, ears, arm, tail and paws each carry an ink edge, and
  the silhouette edges are heavier than the details

### Requirement: The cat uses one accent and no effects

The cat SHALL use one coral accent, on the inner ears, nose, blush and tongue. The
open mouth SHALL be an ink shape with that coral tongue. It SHALL NOT use a
gradient, a cast shadow or a dark underlay.

#### Scenario: One accent

- **WHEN** the cat is licking its paw
- **THEN** the tongue is the coral accent and no pixel of the cat is a gradient or
  a cast shadow

#### Scenario: The light desktop

- **WHEN** the cat is drawn over a light or a dark desktop
- **THEN** the paper fill and the ink outline carry the silhouette with no
  underlay

### Requirement: The face has big eyes, a gaze and a blink

The face SHALL be two large eye whites with ink rings, a vertical ink pupil and a
white highlight in each. A gaze offset SHALL move the pupils with the pose: toward
the panel while peeking or reaching, down at the paw while licking, and at rest
toward the panel. A blink SHALL draw a curved ink stroke in place of the eye, not a
dot.

#### Scenario: The eyes look with the action

- **WHEN** the cat licks its paw
- **THEN** its pupils are below their resting place, toward the tongue

#### Scenario: The blink is an eyelid

- **WHEN** the waiting cat blinks
- **THEN** each eye is a curved ink stroke across the eye's width, not a dot

### Requirement: The face marks stay on the mouth

The face SHALL have a coral nose, an ink smile that ends at the mouth, two coral
blush patches and ink whiskers. The face SHALL rotate about the mouth, so the mouth,
the bubble's tail and its growth origin do not move under the head's tilt.

#### Scenario: The mouth is still the bubble's point

- **WHEN** an answer is on screen in the cat mode
- **THEN** the smile ends on the head's outline at `mouth_at`, and the bubble's
  tail and growth origin use that point

### Requirement: The flat mascot keeps the cat's contracts

The stage, the landing, the paw tangent at `(S, S/2)`, the edge roll, the hop, the
pose deadlines, the spring reach, the irregular lick, the two-beat speak, the tail
sway, the ear twitch and the waiting blink SHALL NOT change. The arm's shoulder
SHALL be hidden by the body fill; the ears' bases SHALL be hidden by the head fill.

#### Scenario: The paw still holds the pad

- **WHEN** the cat has landed at any buffer scale
- **THEN** the paper mitten's right edge is the input card's left rim at that
  card's vertical centre

#### Scenario: No paint outside the stage

- **WHEN** any pose is drawn
- **THEN** the stage's outside corners are transparent and the fills stay inside
  the buffer

#### Scenario: No new idle loop

- **WHEN** the cat is waiting with no request
- **THEN** it is repainted at the idle tick for its blink, not every 16 ms
