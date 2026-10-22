# Spec Delta

## ADDED Requirements

### Requirement: The pointer scrolls the replies

While the pointer is over the replies card — above the bar, inside the panel — a
wheel notch or a two-finger touchpad slide SHALL scroll the transcript: a notch
SHALL move it three lines, a continuous delta SHALL move it by that many pixels,
and a scroll SHALL be clamped to the transcript's ends. Scrolling up SHALL stop
the view following the newest line; reaching the bottom SHALL resume following.
Scrolling over the bar SHALL do nothing. The scroll SHALL NOT resize the panel, nor
change which question the provider is sent.

#### Scenario: A wheel notch over the replies moves three lines

- **WHEN** the pointer is over the replies card and the wheel turns one notch
- **THEN** the view moves three lines toward the direction of the turn

#### Scenario: A touchpad slide scrolls

- **WHEN** two fingers slide up over the replies card
- **THEN** the transcript scrolls by the distance slid, without a notch to quantize it

#### Scenario: Over the bar, nothing scrolls

- **WHEN** the pointer is over the input bar and the wheel turns
- **THEN** the transcript does not move

#### Scenario: Scrolling to the bottom follows again

- **WHEN** the view has been scrolled up and is then scrolled past the newest line
- **THEN** it stops at the newest line and follows new text from then on

### Requirement: The prefix mode is visible in the bar

While the prefix mode waits for its one key, the bar SHALL NOT accept text, SHALL
draw its body in a mode tone distinct from the typing tone, and SHALL show the
instruction `press [ to enter vim mode` in place of the input line, with no caret.
The hint SHALL NOT change the panel's size, and leaving the mode SHALL restore the
bar's tone, its text and its caret.

#### Scenario: The prefix says what it is waiting for

- **WHEN** the user presses `Ctrl+T`
- **THEN** the bar changes tone and reads `press [ to enter vim mode`, and no caret is drawn

#### Scenario: Typing during the prefix does nothing

- **WHEN** a printable key is pressed while the prefix waits
- **THEN** it is swallowed as the prefix's one key and the input line is unchanged

#### Scenario: The bar comes back

- **WHEN** the prefix ends, whether by `[`, `Esc` or any other key
- **THEN** the bar's tone, its typed text and its caret return exactly as they were
