# Spec Delta

## MODIFIED Requirements

### Requirement: The answer's height is capped by the bar's gap

The replies SHALL be allowed to grow to the height of the gap between the bar and
the screen's bottom edge — the same 38 % of the output height that places the bar —
and no further. The panel SHALL NOT exceed that gap plus the bar plus the 12 px
between the cards, and SHALL NOT exceed the output height: where the gap is nearly
the whole screen, the space above the bar clamps the cap instead. When the replies
need more than the cap, the panel SHALL stay at the cap and scroll inside it,
keeping the existing highlight, paging and `Alt+C` behaviour. Because dragging the
panel changes the gap, the cap SHALL be recomputed from the gap in force at that
moment.

#### Scenario: A long answer hits the cap and scrolls

- **WHEN** the replies are taller than the gap below the bar
- **THEN** the panel stops at that height, the newest rows stay visible, and scrolling reaches the older ones

#### Scenario: Dragged near the top

- **WHEN** the panel is dragged so that the gap below the bar is most of the output
- **THEN** the cap is the room left above the bar instead, so the panel never leaves the output

#### Scenario: No output height yet

- **WHEN** the output's logical height is not known yet
- **THEN** the panel uses the gap of a 1080-pixel output and re-sizes once the real height is known

### Requirement: The panel is summoned, focused, and now movable

The panel SHALL be summoned by a compositor keybinding that runs `aside`, and
SHALL take keyboard focus on appearing. It is a layer surface: the compositor
places it by the anchor and margin. It SHALL also be **movable by the user**: a
press of the left button held in place and then moved SHALL carry the panel with
the pointer, staying inside the output and keeping its visible gap at the bottom.
The position SHALL live only as long as the process: it is not written anywhere
and is not restored on the next run. The panel MUST NOT be tiled or decorated by
the compositor. When the compositor closes the surface or the connection, the
process SHALL exit as the failure rules say: reported and non-zero before the
panel appeared, quietly after it did.

#### Scenario: Panel appears focused

- **WHEN** the keybinding runs `aside` with a valid credential
- **THEN** the panel appears where its anchor puts it, and receives typed text

#### Scenario: Dragged by the field

- **WHEN** the user holds the left button for a quarter of a second and moves the pointer
- **THEN** the panel follows the pointer, and a shorter click moves nothing

#### Scenario: Nothing is written

- **WHEN** the panel has been dragged and `aside` exits
- **THEN** nothing on disk records where it was, and the next run starts in its anchored place

#### Scenario: No layer-shell support

- **WHEN** the compositor does not advertise `zwlr_layer_shell_v1`
- **THEN** `aside` reports that it cannot place its panel and exits non-zero without showing a window

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
| `j` / `k` | vim | move the highlight one line down / up |
| `n` / `p` | vim | move it one page down / up |
| `h` / `l` | vim | move it to the first / last line |
| `Ctrl+j` / `Ctrl+k` | vim | move it one page down / up |
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

- **WHEN** the user presses `j` in vim mode
- **THEN** the highlight moves down and the input line is unchanged

#### Scenario: Escape during a stream cancels rather than closes

- **WHEN** the user presses `Esc` in typing mode while deltas are still arriving
- **THEN** the stream is cancelled, the panel stays open, and the partial reply remains visible and marked as cancelled

#### Scenario: Escape when idle closes

- **WHEN** the user presses `Esc` in typing mode with no stream open
- **THEN** the process exits

#### Scenario: Enter sends

- **WHEN** the input contains `why` and the user presses `Enter`
- **THEN** a request is sent with `why` as the newest turn, and the input is cleared

## ADDED Requirements

### Requirement: A prefix mode dispatches settings keys

`Ctrl+T` SHALL enter a prefix mode that waits for exactly one key and then
returns to typing, the way a terminal multiplexer's prefix does. In this change the
prefix SHALL map `[` to vim mode and every other key, `Esc` included, to nothing;
the table SHALL be a dispatch site so later settings commands are added there
rather than on new keys. The prefix mode SHALL NOT change the panel's geometry or
its text, and a key consumed by it MUST NOT reach the input line.

#### Scenario: The prefix leads to vim mode

- **WHEN** the user presses `Ctrl+T` and then `[`
- **THEN** vim mode is on, and neither key reached the input line

#### Scenario: An unknown prefix key does nothing

- **WHEN** the user presses `Ctrl+T` and then `z`
- **THEN** the panel is back in typing mode, no `z` was typed, and nothing else changed

#### Scenario: The prefix is not sticky

- **WHEN** the user presses `Ctrl+T`, then `Esc`, then types `hi`
- **THEN** the input line holds `hi`, because the prefix took one key and left

### Requirement: Vim mode moves the highlight and says so

While vim mode is on the panel SHALL draw `-- VIM --` in the bar, right-aligned,
in the secondary ink, and SHALL NOT draw the caret. Vim mode SHALL take the
keyboard from the input line and from the input method: no key MAY append or
delete typed text, and no pre-edit MAY be open. Leaving the mode SHALL restore
both and redraw the caret. The mode SHALL NOT close by itself, and SHALL NOT
change the panel's size, the wrapping, or what the provider is sent.

#### Scenario: The mode is visible

- **WHEN** vim mode is entered
- **THEN** the bar reads `-- VIM --` and the caret is gone, and both revert on `Esc`

#### Scenario: An input method cannot type into vim mode

- **WHEN** an input method commits text while vim mode is on
- **THEN** the input line is unchanged

#### Scenario: Sending from vim mode

- **WHEN** the user presses `Enter` in vim mode with text in the input line
- **THEN** the question is sent exactly as `Enter` in typing mode would send it
