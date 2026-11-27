# Spec Delta

## ADDED Requirements

### Requirement: The cat uses Notion's drawing palette

The stage SHALL draw the cat in Notion's dark-mode ink, `#e9e9e7`, with
`#ffa344` as its only warm accent, on the tongue and the open mouth. The drawing
SHALL be flat: no gradient and no cast shadow. A single dark underlay of at most
30 % alpha MAY sit under the ink so the cat stays visible on a light desktop.

#### Scenario: One line, one accent

- **WHEN** the cat is licking its paw
- **THEN** the contour is drawn in the off-white ink, the tongue is the orange
  accent, and no pixel of the cat is a gradient or a cast shadow

### Requirement: The line is a Notion marker line

The line SHALL be one weight of about 1.2 % of the stage's side, with rounded
caps, and SHALL carry a small deterministic hand-drawn waver that is zero at
every join and at the paw contact.

#### Scenario: Joins stay exact

- **WHEN** the cat is drawn at any stage size
- **THEN** the body meets the head's ring, the tail meets the body, and the leg
  meets the paw without a waver offset at those points

### Requirement: The cat's face and paw are Notion-simple

The face SHALL be two dot eyes, one small filled nose, one curved smile and two
whiskers. The paw SHALL be a rounded mitten: an ellipse ring with two toe strokes,
its right edge still tangent to the input card's left rim at that card's vertical
centre. The mouth SHALL stay a point on the head's outline, and the bubble's tail
and growth origin SHALL still use it.

#### Scenario: The paw is a mitten on the pad

- **WHEN** the cat has landed
- **THEN** the mitten's right edge is the input card's left rim at its vertical
  centre

#### Scenario: The face is two dots and a smile

- **WHEN** the cat is waiting
- **THEN** its face is drawn as two eye dots, a filled nose, one smile curve and
  two whiskers, with no other facial fill or outline

#### Scenario: The bubble still comes out of the mouth

- **WHEN** an answer is on screen in the cat mode
- **THEN** the card-coloured tail ends on the head's outline at the mouth, and
  the growth starts from that point
