# Spec Delta

## MODIFIED Requirements

### Requirement: The keyboard contract

The panel SHALL give these keys these meanings, and nothing else:

| Key | Mode | Action |
|---|---|---|
| `Enter` | typing | send the typed question |
| `Esc` | typing | cancel an open stream; clear a pointer selection; close the panel when neither is open |
| `Esc` | prefix | leave prefix mode, doing nothing else |
| `Esc` | copy | clear the selection; leave copy mode when there is none |
| `Ctrl+T` | typing | enter the prefix mode |
| `[` | prefix | enter copy mode |
| `Alt+h` / `Alt+j` / `Alt+k` / `Alt+l` | any | move the **panel** 16 px left / down / up / right |
| `Alt+Ctrl+hjkl` | any | the same directions, 160 px |
| `Up` / `Down` | typing | move the highlighted transcript line |
| `PageUp` / `PageDown` | typing | move it one page |
| `Alt+C` / `Ctrl+C` | typing, copy | copy the selection, or the highlighted line |
| `Ctrl+V` | typing | paste the clipboard into the input line |
| copy-mode keys | copy | as the copy-mode requirement lists them |
| any other printable key | prefix, copy | nothing: the input line does not change |

The input SHALL be one logical line that soft-wraps, edited with the standard
keys. There SHALL be no newline key. Every action SHALL be reachable without a
pointer, and every pointer action SHALL have a keyboard equivalent.

#### Scenario: Copy takes one line

- **WHEN** the user presses `Ctrl+C` with a line highlighted and nothing selected
- **THEN** the clipboard holds that line and nothing else

#### Scenario: Escape still means what it meant

- **WHEN** the user presses `Esc` in typing mode while deltas are still arriving
- **THEN** the stream is cancelled, the panel stays open

#### Scenario: A letter does not type while copy mode is on

- **WHEN** the user presses `j` in copy mode
- **THEN** the cursor moves down a line and the input line is unchanged

## ADDED Requirements

### Requirement: Copy mode walks the replies and takes what it selects

`Ctrl+T` then `[` SHALL enter copy mode, which SHALL draw a visible cursor in the
replies and take the keyboard from the input line and from the input method. The
cursor SHALL move by:

| Key | Moves the cursor |
|---|---|
| `h` / `l` | one character left / right |
| `0` / `$` | the start / end of the line |
| `j` / `k` | one line down / up |
| `w` / `b` | to the start of the next / previous word, crossing lines |
| `Ctrl+d` / `Ctrl+u` | half a page down / up |
| `Ctrl+f` / `Ctrl+b`, `PageDown` / `PageUp` | one page down / up |
| `g` / `G` | the first / last line |

`Space` (or `v`) SHALL start a selection at the cursor; moving the cursor SHALL
extend it. `Enter` or `y` SHALL copy the selection — or, with no selection, the
line the cursor is on — SHALL exit copy mode, and SHALL leave `copied N lines` in
the bar for about a second. `q` SHALL exit copy mode. `Esc` SHALL clear the
selection and leave the mode when there is none. The panel SHALL NOT resize, the
input line MUST NOT change, and the cursor and selection SHALL be the same objects
a pointer drags, so the two ways of choosing text agree.

#### Scenario: Walking and copying two lines

- **WHEN** the user enters copy mode, presses `j` twice, `Space`, `j`, then `Enter`
- **THEN** the clipboard holds those two lines, and copy mode has ended

#### Scenario: A word at a time

- **WHEN** the user presses `w` three times from the start of a line
- **THEN** the cursor sits at the start of that line's third word, or on a later line when the line runs out

#### Scenario: The cursor says where it is

- **WHEN** copy mode is entered
- **THEN** a cursor is drawn at the highlighted line, the bar reads `-- COPY --`, and the caret is not drawn

#### Scenario: Leaving without copying

- **WHEN** the user presses `q` in copy mode
- **THEN** the mode ends, the clipboard is untouched, and the bar returns to what it was

## REMOVED Requirements

### Requirement: Vim mode moves the highlight and says so

**Reason**: Copy mode does what vim mode did to the replies, with the keys tmux uses
for it, and the window is moved by a chord that needs no mode.

**Migration**: `Ctrl+T [` now enters copy mode; `hjkl` move a cursor rather than the
panel; the panel moves with `Alt+hjkl` from any mode.

### Requirement: Vim mode steps the window

**Reason**: Moving the panel does not need a mode.

**Migration**: `Alt+hjkl` moves it 16 px and `Alt+Ctrl+hjkl` 160 px, in every mode.
