## ADDED Requirements

### Requirement: Only the cards take pointer input

The layer surface SHALL set its input region to the two cards it draws — the replies
card and the bar — and to nothing else. The 26 px shadow margin around them and the
16 px gap between them SHALL pass pointer input through to whatever is below, so a
click on a window seen there focuses that window rather than the panel. The region
SHALL be recomputed whenever the cards' geometry changes, so it never covers space
the panel is not drawing.

#### Scenario: A click in the shadow margin

- **WHEN** the pointer clicks a window visible in the panel's transparent shadow margin
- **THEN** the click reaches that window, which becomes focused and typable, and the panel neither takes the click nor moves

#### Scenario: A click in the gap between the cards

- **WHEN** the pointer clicks a window visible in the 16 px between the replies card and the bar
- **THEN** the click reaches that window and the panel does not start a drag

#### Scenario: A click on a card

- **WHEN** the pointer presses on a reply line or the bar
- **THEN** the panel takes it as before: selection on a reply, drag on the bar
