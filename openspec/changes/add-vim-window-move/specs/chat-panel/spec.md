# Spec Delta

## MODIFIED Requirements

### Requirement: The keyboard contract

The panel SHALL give these keys these meanings, and nothing else:

| Key | Mode | Action |
|---|---|---|
| `Enter` | typing, vim | send the typed question |
| `Esc` | typing | cancel an open stream; close the panel when no stream is open |
| `Esc` | vim | leave vim mode, cancelling nothing |
| `Esc` | prefix | leave prefix mode, doing nothing else |
| `Ctrl+T` | typing | enter the prefix mode |
| `[` | prefix | enter vim mode |
| `h` / `j` / `k` / `l` | vim | move the **panel** 16 px left / down / up / right |
| `Ctrl+h` / `Ctrl+j` / `Ctrl+k` / `Ctrl+l` | vim | the same directions, 160 px |
| `n` / `p` | vim | move the highlight one page down / up |
| `Ctrl+n` / `Ctrl+p` | vim | move it one line down / up |
| `g` / `G` | vim | move it to the first / last line |
| `Up` / `Down` | typing | move the highlight one line |
| `PageUp` / `PageDown` | typing | move it one page |
| `Alt+C` | typing, vim | copy the highlighted line |
| any other printable key | vim, prefix | nothing: the input line does not change |

The input SHALL be one logical line that soft-wraps, edited with the standard
keys — `Left`, `Right`, `Home`, `End`, `Backspace`, `Delete`, and
`Ctrl+Left`/`Ctrl+Right` for word movement. There SHALL be no newline key: `Enter`
always sends, and a multi-line question can still be pasted. Every action SHALL be
reachable without a pointer. No key SHALL have two meanings in the same mode, and
`Up` and `Down` SHALL always mean the transcript highlight in typing mode.

#### Scenario: Escape in vim mode does not cancel

- **WHEN** the user presses `Esc` while vim mode is on and a stream is arriving
- **THEN** vim mode ends, the stream continues, and a second `Esc` cancels it

#### Scenario: Letters do not type in vim mode

- **WHEN** the user presses `h` in vim mode
- **THEN** the panel moves left and the input line is unchanged

#### Scenario: Escape during a stream cancels rather than closes

- **WHEN** the user presses `Esc` in typing mode while deltas are still arriving
- **THEN** the stream is cancelled, the panel stays open, and the partial reply remains visible and marked as cancelled

#### Scenario: Escape when idle closes

- **WHEN** the user presses `Esc` in typing mode with no stream open
- **THEN** the process exits

#### Scenario: Enter sends

- **WHEN** the input contains `why` and the user presses `Enter`
- **THEN** a request is sent with `why` as the newest turn, and the input is cleared

### Requirement: The panel is summoned, focused, and now movable

The panel SHALL be summoned by a compositor keybinding that runs `aside`, and
SHALL take keyboard focus on appearing. It is a layer surface: the compositor
places it by the anchor and margin. It SHALL also be **movable by the user**, in
three ways that all move the same thing: a press of the left button held in place
and then moved, and — in vim mode — `hjkl` with `Ctrl+hjkl` for larger steps. The
panel SHALL stay inside the output on all four sides however it moves, and the
position SHALL live only as long as the process: it is not written anywhere and is
not restored on the next run. The panel MUST NOT be tiled or decorated by the
compositor. When the compositor closes the surface or the connection, the process
SHALL exit as the failure rules say: reported and non-zero before the panel
appeared, quietly after it did.

#### Scenario: Panel appears focused

- **WHEN** the keybinding runs `aside` with a valid credential
- **THEN** the panel appears where its anchor puts it, and receives typed text

#### Scenario: Dragged by the field

- **WHEN** the user holds the left button for a quarter of a second and moves the pointer
- **THEN** the panel follows the pointer, and a shorter click moves nothing

#### Scenario: Stepped by the keyboard

- **WHEN** the user presses `h` ten times in vim mode
- **THEN** the panel has moved 160 px left, or as far as the output's edge allows

#### Scenario: Nothing is written

- **WHEN** the panel has been moved and `aside` exits
- **THEN** nothing on disk records where it was, and the next run starts in its anchored place

#### Scenario: No layer-shell support

- **WHEN** the compositor does not advertise `zwlr_layer_shell_v1`
- **THEN** `aside` reports that it cannot place its panel and exits non-zero without showing a window

### Requirement: Vim mode moves the highlight and says so

While vim mode is on the panel SHALL draw `-- VIM --` in the bar, right-aligned,
in the secondary ink, and SHALL NOT draw the caret. Vim mode SHALL take the
keyboard from the input line and from the input method: no key MAY append or
delete typed text, and no pre-edit MAY be open. Leaving the mode SHALL restore
both and redraw the caret. The mode SHALL NOT close by itself, and SHALL NOT
change the panel's size, the wrapping, or what the provider is sent — except that
moving the panel with `hjkl` changes the gap below the bar, and with it the height
cap the replies are allowed. Moving the panel SHALL NOT re-wrap the transcript,
because the width does not change.

#### Scenario: The mode is visible

- **WHEN** vim mode is entered
- **THEN** the bar reads `-- VIM --` and the caret is gone, and both revert on `Esc`

#### Scenario: An input method cannot type into vim mode

- **WHEN** an input method commits text while vim mode is on
- **THEN** the input line is unchanged

#### Scenario: Sending from vim mode

- **WHEN** the user presses `Enter` in vim mode with text in the input line
- **THEN** the question is sent exactly as `Enter` in typing mode would send it

#### Scenario: A step never re-wraps

- **WHEN** the panel is moved with `hjkl` while an answer is on screen
- **THEN** every line keeps the wrapping it had, and only the panel's position changed

## ADDED Requirements

### Requirement: Vim mode steps the window

`h`, `j`, `k` and `l` SHALL move the panel on the output, 16 logical px per press
in that direction, and `Ctrl+h`/`Ctrl+j`/`Ctrl+k`/`Ctrl+l` SHALL move it ten times
that. The move SHALL be applied through the same clamped path as the drag, so the
panel cannot leave the output on any side, and the height cap SHALL follow the gap
the move produced. A press that cannot move — already against an edge — SHALL
change nothing and SHALL NOT count as a change to the panel.

#### Scenario: Steps go where they say

- **WHEN** the user presses `l` three times
- **THEN** the panel's left edge is 48 px further right, or against the output's edge

#### Scenario: Ctrl takes a longer step

- **WHEN** the user presses `Ctrl+k`
- **THEN** the panel moves up 160 px, or as far as the space above it allows

#### Scenario: The cap follows the step

- **WHEN** the panel is stepped upwards while an answer is on screen
- **THEN** the gap below the bar grows, and the replies may grow with it up to the room above
