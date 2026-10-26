# Spec Delta

## ADDED Requirements

### Requirement: The text size changes from the keyboard

The prefix mode SHALL carry the text size: `+` or `=` SHALL make the panel's text two
logical pixels larger and `-` two pixels smaller, within the same `8..=72` range the
configuration accepts. A press at either limit SHALL change nothing and SHALL NOT fail.
The prefix's hint on the bar SHALL name both keys, so the setting is discoverable
without the README.

The new size SHALL take effect in the same run: the font is read again at that size, the
bar, the row height and the gaps follow it, the replies on screen are laid out again at
the new metrics, and the surface is resized once. The size SHALL NOT be written
anywhere and SHALL NOT be restored on the next run; the configuration file is read, never
written, and the panel's "nothing on disk" rule is unchanged.

#### Scenario: Larger text

- **WHEN** the panel is at 20 px and the user presses `Ctrl+T` then `+`
- **THEN** the panel's text, bar and rows are drawn at 22 px on the next frame, and the input line keeps its text

#### Scenario: The limits hold

- **WHEN** the panel is at 72 px and the user presses `Ctrl+T` then `+`
- **THEN** nothing changes, nothing is reported as an error, and the panel keeps working

#### Scenario: Smaller text, then a follow-up

- **WHEN** the user shrinks the text while an answer is on screen and then asks a follow-up
- **THEN** the answer is re-wrapped at the smaller size and the follow-up is drawn in it

#### Scenario: Only the prefix carries the size keys

- **WHEN** the panel is typing and the user presses `+` or `-`
- **THEN** they are typed into the input line as characters, and the size does not change
