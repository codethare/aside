# Spec Delta

## MODIFIED Requirements

### Requirement: Typed input accepts IME-composed text

The panel SHALL act as a `zwp_text_input_v3` client so that a running input method
(fcitx, ibus) can deliver composed text, including multi-character and CJK
commits, pre-edit strings and cursor position. Text-input manager being advertised
is NOT proof that an input method is running. The panel MUST arbitrate **per
keystroke**, not once per session: a text key pressed while an input method is in
play SHALL be held until the end of the current event round, and:

- if the input method speaks in that round — a pre-edit, a commit, a deletion or a
  `done` — the held key was its business and MUST be dropped;
- if it stays silent, the held key's text SHALL be inserted, so letters and digits
  keep working with the input method switched off or gone.

A key pressed while a pre-edit is open SHALL type nothing. The panel MUST NOT
refuse to start when the manager is missing.

#### Scenario: A composed Chinese question arrives intact

- **WHEN** the user composes `什么是 EAGAIN` with an IME and commits it
- **THEN** the input line holds exactly that text and pre-edit text is rendered while it is being composed

#### Scenario: English after the input method is switched off

- **WHEN** the user has typed Chinese and then switches the input method off
- **THEN** letters and digits typed afterwards reach the input line

#### Scenario: An input method that keeps composing is not typed over

- **WHEN** a text key is pressed and the input method answers with a pre-edit in the same round
- **THEN** the key's own character does not appear in the input line

#### Scenario: A manager with no input method behind it

- **WHEN** the compositor advertises `zwp_text_input_manager_v3` but no input method is connected, so nothing is ever committed
- **THEN** directly typed characters still reach the input line
