# Spec Delta

## MODIFIED Requirements

### Requirement: The panel is summoned, focused, and movable on both axes

The panel SHALL be summoned by a compositor keybinding that runs `aside`, and
SHALL take keyboard focus on appearing. It is a layer surface, anchored to the
bottom and left edges so that both margins are real positions. It SHALL be
**movable by the user** in three ways that all move the same thing: the left button
pressed on the panel and moved, — in vim mode — `hjkl` with `Ctrl+hjkl` for larger
steps. A drag SHALL follow the pointer from its first motion: it MUST NOT wait for
a hold, and a press released without moving SHALL move nothing. Horizontal and
vertical movement SHALL both take effect, the panel SHALL stay inside the output on
all four sides however it moves, and moving it MUST NOT resize it. The position
SHALL live only as long as the process: it is not written anywhere and is not
restored on the next run.

#### Scenario: The panel follows the pointer

- **WHEN** the user presses the left button on the panel's bar and moves the pointer
- **THEN** the panel moves with the pointer from that first motion, with no delay

#### Scenario: A click moves nothing

- **WHEN** the user presses and releases the left button without moving it
- **THEN** the panel is where it was

#### Scenario: Nothing is written

- **WHEN** the panel has been dragged and `aside` exits
- **THEN** nothing on disk records where it was, and the next run starts in its anchored place

### Requirement: The keyboard contract

The panel SHALL give these keys these meanings, and nothing else:

| Key | Mode | Action |
|---|---|---|
| `Enter` | typing, vim | send the typed question |
| `Esc` | typing | cancel an open stream; clear a pointer selection; close the panel when neither is open |
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
| `Alt+C` / `Ctrl+C` | typing, vim | copy the selection, or the highlighted line |
| `Ctrl+V` | typing | paste the clipboard into the input line |
| any other printable key | vim, prefix | nothing: the input line does not change |

The input SHALL be one logical line that soft-wraps, edited with the standard
keys. There SHALL be no newline key. Every action SHALL be reachable without a
pointer, and every pointer action SHALL have a keyboard equivalent.

#### Scenario: Copy takes one line

- **WHEN** the user presses `Ctrl+C` with a line highlighted and nothing selected
- **THEN** the clipboard holds that line and nothing else

#### Scenario: Escapes still mean what they meant

- **WHEN** the user presses `Esc` in typing mode while deltas are still arriving
- **THEN** the stream is cancelled, the panel stays open

### Requirement: The transcript draws only the current turn

The replies box SHALL show what the model or the panel said about the question
just asked, and SHALL NOT draw the answers of earlier turns. The highlighted
line, or the lines a pointer selected, SHALL be drawn in the highlight ink.

#### Scenario: Only the reply is on screen

- **WHEN** a finished question and its answer are on screen
- **THEN** the answer's lines are there and the question appears nowhere in the panel

## ADDED Requirements

### Requirement: Reply lines can be selected with the pointer

A press of the left button on a reply line SHALL anchor a selection at that line;
motion with the button down SHALL extend it over every line between, releasing
SHALL leave it in place, and pressing again SHALL replace it. The selection SHALL
cover whole lines, because a line is what the panel copies and what its keyboard
highlight names. A press that is not on a reply line SHALL NOT start a selection,
and SHALL move the panel instead if it moves. Lines inside the selection SHALL be
drawn in the highlight ink, without moving any text.

`Alt+C` and `Ctrl+C` SHALL copy the selected lines, one per line, in the order they
appear. `Esc` SHALL clear the selection before it cancels a stream or closes the
panel, sending a question SHALL clear it, and a selection MUST NOT survive a turn
change: the lines it named are no longer on screen.

#### Scenario: Selecting and copying three lines

- **WHEN** the user presses on the first line of an answer, drags to its third line, releases, and presses `Ctrl+C`
- **THEN** the clipboard holds those three lines, one per line, and nothing else

#### Scenario: Escape clears before it closes

- **WHEN** a selection exists and the user presses `Esc` with no stream open
- **THEN** the selection is cleared and the panel stays open; a second `Esc` closes it

#### Scenario: A selection cannot outlive its turn

- **WHEN** the user sends a question with a selection on screen
- **THEN** the selection is gone, and `Ctrl+C` copies the newly highlighted line only
