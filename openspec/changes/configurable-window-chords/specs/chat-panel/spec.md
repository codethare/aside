# Spec Delta

## MODIFIED Requirements

### Requirement: The keyboard contract

The panel SHALL give these keys these meanings, and nothing else:

| Key | Mode | Action |
| --- | --- | --- |
| `Enter` | typing | send the typed question |
| `Esc` | typing | cancel an open stream; clear a pointer selection; close the panel when neither is open |
| `Esc` | prefix | leave prefix mode, doing nothing else |
| `Esc` | copy | clear the selection; leave copy mode when there is none |
| `Ctrl+T` | typing | enter the prefix mode |
| `[` | prefix | enter copy mode |
| the **window chords** | any | move the **panel** by the configured step in the chord's direction, clamped to the output |
| `Up` / `Down` | typing | move the highlighted transcript line |
| `PageUp` / `PageDown` | typing | move it one page |
| `Alt+C` / `Ctrl+C` | typing, copy | copy the selection, or the highlighted line |
| `Ctrl+V` | typing | paste the clipboard into the input line |
| copy-mode keys | copy | as the copy-mode requirement lists them |
| any other printable key | prefix, copy | nothing: the input line does not change |

The input SHALL be one logical line that soft-wraps, edited with the standard
keys. There SHALL be no newline key. Every action SHALL be reachable without a
pointer, and every pointer action SHALL have a keyboard equivalent.

The window chords SHALL be configurable in the configuration file, one chord per
direction, each one or more of `alt`, `ctrl` and `shift` followed by a key the panel
can see, plus a step in logical pixels. Unconfigured, they SHALL be `Alt+h`,
`Alt+j`, `Alt+k` and `Alt+l` moving 16 px, and the same chords with `ctrl` added
moving ten times as far. The chords SHALL be honoured in every mode, copy mode
included, and in copy mode SHALL be consulted before that mode's own table.

A configured chord that carries no modifier, that uses `super`, that repeats another
direction, that names more than one key, or that takes a key the panel already owns
SHALL be refused: the panel SHALL report it with the configuration file and the line,
and SHALL NOT open. A step outside 1 to 512 logical pixels SHALL be refused the same
way.

#### Scenario: Copy takes one line

- **WHEN** copy mode is active and a line has been named
- **THEN** the chord copies that line and the input line is unchanged

#### Scenario: The panel moves while reading

- **WHEN** copy mode is active and a window chord is pressed
- **THEN** the panel moves and the copy cursor does not

#### Scenario: A chord the panel cannot honour

- **WHEN** the configuration names a window chord without a modifier, with `super`, or one the panel already uses
- **THEN** nothing opens, and the message names the file and the line

#### Scenario: A configured chord

- **WHEN** the configuration sets `window_left = "ctrl+shift+h"` and `window_step = "24"`
- **THEN** that chord moves the panel 24 px left in every mode, and the same chord with `ctrl` added moves it ten times as far
