# Spec Delta

## ADDED Requirements

### Requirement: The cat is drawn as unfilled line art

The stage SHALL draw the cat as unfilled strokes of one weight, proportional to
the stage's side, in the panel's light ink. It SHALL NOT paint a filled
silhouette, a filled head or filled ears. A dark stroke at a small offset SHALL
lie under the light strokes so the drawing stays legible on a light desktop. The
face SHALL be two eye dots, a nose, a mouth and two whiskers.

#### Scenario: The cat is a drawing, not a silhouette

- **WHEN** the cat is in the waiting pose
- **THEN** the body's interior and the head's centre are transparent, and the
  body's outline, the head's ring and the paw are inked

#### Scenario: Frames still differ by pose

- **WHEN** the lick or the mouth is animated
- **THEN** successive frames of that pose differ in the tongue or the mouth, and
  no frame paints outside the stage's buffer

### Requirement: The line art keeps the paw contact

The cat SHALL be composed of a head ring, two open ear lines, a body contour open
at the head, a tail curve, a front leg and a paw ring. The paw ring SHALL be
tangent to the stage's right edge at the input card's vertical centre, so the paw
still meets the pad.

#### Scenario: The paw still holds the pad

- **WHEN** the cat has landed at any buffer scale
- **THEN** the paw ring reaches the stage's right edge at the input card's
  vertical centre, which is the card's left rim

### Requirement: The bubble comes out of the mouth on the outline

The cat's mouth SHALL be a point on the head's outline. The bubble's
card-coloured tail SHALL end at that point without crossing the inside of the
head, and that point SHALL remain the bubble's growth origin.

#### Scenario: The bubble comes out of the mouth

- **WHEN** an answer is on screen in the cat mode
- **THEN** the card-coloured tail ends on the head's outline at the mouth, and
  the growth starts from that same point
