# Spec Delta

## ADDED Requirements

### Requirement: The panel's look is a configuration key

`aside` SHALL read `mode` from its configuration file, accepting `"default"` and
`"cat"` and refusing anything else with the file and the line that carried it, and no
panel. With the key absent the mode SHALL be `"default"`, and `"default"` SHALL draw
exactly the panel that the key's absence drew before this change: the two plush cards,
the plush chrome, the plush rim.

The key SHALL be read once, at startup, and SHALL NOT change the panel's sizes, its
keys, its scrolling, its selection, its markdown, its request, or its sandbox.

#### Scenario: A value that is not a mode is refused

- **WHEN** the configuration file contains `mode = "dog"`
- **THEN** one line names the file, the line and the two accepted values, and no panel opens

#### Scenario: Absent means the plush panel

- **WHEN** the configuration file has no `mode` line
- **THEN** the panel is drawn exactly as it is drawn today

### Requirement: In the cat mode the input is the cat's paw

With `mode = "cat"` the input card SHALL be drawn as the magnified paw pad: the bar's
own box, height, padding, caret, marquee, hint and scroll, with the pad's shape and rim
in place of the plush card's. The text SHALL be laid out at the same width and the same
size as in the default mode, the input SHALL accept text, IME preedit, paste, the caret
and every key of the key table from the first frame, and the pad SHALL NOT wait for any
part of the cat's arrival.

#### Scenario: Typing before the cat has arrived

- **WHEN** the panel is summoned in the cat mode and a key is pressed in the first frame
- **THEN** the character is in the pad, with the caret after it, whatever the cat is doing

#### Scenario: The pad keeps the bar's box

- **WHEN** the panel is in the cat mode at any size
- **THEN** the input card's width and height are the default mode's at that size, and the text wraps at the same width

### Requirement: In the cat mode the answer is the cat's bubble

With `mode = "cat"` the reply card SHALL be the cat's bubble: the same fixed box above
the input, the same rows, markdown, tables, scroll, selection, copy and scroll
indicator. Where there is a stage it SHALL also carry a tail from the bubble's
bottom-left corner toward the cat's mouth, and its growth SHALL start at the mouth
rather than at the bar; where there is no stage it SHALL have no tail and SHALL grow
from the bar as the default mode's does.

The bubble SHALL stay on screen while the next request runs, showing the answer that
was on screen when the question was sent, and SHALL be replaced by the new answer when
its first text arrives. An answer that ends SHALL NOT remove the bubble.

#### Scenario: The bubble outlives the next question

- **WHEN** an answer is on screen in the cat mode and the reader sends a new question
- **THEN** the previous answer stays in the bubble while the request runs, and the new answer replaces it at its first text

#### Scenario: A first question has no previous bubble

- **WHEN** the first question of a run is sent in the cat mode
- **THEN** the panel is the pad alone until the answer's first text arrives

#### Scenario: The bubble's text is the panel's text

- **WHEN** an answer with a fenced block and a table is on screen in the cat mode
- **THEN** it is drawn as the default mode draws it, with the same inks, the same wrapping and the same copy behaviour
