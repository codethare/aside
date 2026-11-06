# Spec Delta

## ADDED Requirements

### Requirement: The panel's box resizes without resizing its text

The panel SHALL have a box size from 1.0 that scales **only** its width and the room
the replies are given. It MUST NOT change the text's size, the row height, the bar's
height, the padding, the gap between the cards, the corner radius or the shadow
margin. A bigger box SHALL therefore show more of the same text, and the transcript
SHALL be wrapped again at the new width.

The box size SHALL be reachable three ways: `Ctrl+Alt+minus` and `Ctrl+Alt+equal` or
`Ctrl+Alt+plus` in every mode; `Ctrl` with the wheel over the panel; and a two-finger
pinch with the pointer on the panel, which SHALL follow the gesture continuously.
`Ctrl+Alt+BackSpace` SHALL return the box to its design size. The wheel without
`Ctrl` SHALL keep its present meaning.

The box SHALL be clamped to a fixed range and to what the output can hold, and the
size SHALL live only as long as the process.

#### Scenario: A bigger box shows more, not bigger

- **WHEN** the box is made bigger while an answer is on screen
- **THEN** the panel is wider and shows more rows, the text is the same size as before, and the lines wrap at the new width

#### Scenario: Every mode has the keys

- **WHEN** `Ctrl+Alt+plus` is pressed in typing, prefix or copy mode
- **THEN** the box grows, and nothing is typed and no copy-mode motion happens

#### Scenario: Back to the default

- **WHEN** `Ctrl+Alt+BackSpace` is pressed after any number of resizes
- **THEN** the box is the size it starts at

#### Scenario: The output is the limit

- **WHEN** the box is made bigger on an output that cannot hold it
- **THEN** it stops at the size that fits, and nothing is drawn outside the output
