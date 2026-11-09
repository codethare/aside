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
| `Esc` | editor, insert | change to normal |
| `Esc` | editor, normal | leave the editor, keeping the text |
| `Ctrl+T` | typing | enter the prefix mode |
| `Ctrl+G` | typing | open the editor on the input line |
| `Ctrl+G` | editor | leave the editor, keeping the text |
| `[` | prefix | enter copy mode |
| `Alt+h` / `Alt+j` / `Alt+k` / `Alt+l` | any | move the **panel** 16 px left / down / up / right |
| `Alt+Ctrl+hjkl` | any | the same directions, 160 px |
| `Up` / `Down` | typing | move the highlighted transcript line |
| `PageUp` / `PageDown` | typing | move it one page |
| `Alt+C` / `Ctrl+C` | typing, copy | copy the selection, or the highlighted line |
| `Ctrl+V` | typing, editor insert | paste the clipboard into the input line or the editor |
| copy-mode keys | copy | as the copy-mode requirement lists them |
| editor keys | editor | as the editor requirement lists them |
| any other printable key | prefix, copy | nothing: the input line does not change |

In typing mode the input SHALL be one logical line that soft-wraps, edited with the
standard keys, and there SHALL be no newline key there. The input MAY hold newlines
— a paste can carry them, and the editor writes them — and `Enter` sends the whole
text.

Every action SHALL be reachable without a pointer, and every pointer action SHALL
have a keyboard equivalent.

#### Scenario: Copy takes one line

- **WHEN** `Alt+C` is pressed with the highlighted line inside a card
- **THEN** the clipboard holds that line's own text and nothing from the card around it

#### Scenario: The prefix is one key wide

- **WHEN** `Ctrl+T` is pressed and then a key that is not `[`
- **THEN** the prefix is left, that key does nothing, and the input line is unchanged

#### Scenario: A newline only exists inside the editor

- **WHEN** `Enter` is pressed in typing mode
- **THEN** the question is sent
- **WHEN** `Enter` is pressed in the editor's insert mode
- **THEN** a line break is inserted and nothing is sent

## ADDED Requirements

### Requirement: The input bar opens a temporary editor

`Ctrl+G` SHALL turn the input bar into a modal editor over the same surface, holding
the input line's text, with a caret that moves through it by vim's keys. The editor
SHALL NOT write the text anywhere, SHALL NOT start a process, and SHALL NOT depend on
a file: the text lives in the panel's memory for as long as the mode is open.

The bar SHALL grow to the editor's rows — at most eight lines of buffer, plus one row
for the command line while it is being typed — and the panel SHALL grow with it,
keeping its bottom edge and the replies card above it where they were. Leaving the
editor SHALL return the bar and the surface to the height they had.

The editor SHALL start in insert mode, and SHALL show which of its modes is on in the
bar's label. In insert, `Enter` SHALL insert a line break and `Esc` SHALL change to
normal. In normal, the moves, the character and line deletions, the ways into insert,
and the command line SHALL behave as this change's design lists them: `h l j k`,
`0 $`, `w b`, `x`, `dd`, `i a A I`, `o O`, `gg G`, `:`, `Esc`.

`:wq`, `:q` and `:x` SHALL put the buffer back into the input line and leave; `:q!`
SHALL leave and keep what the input line held before the editor opened; any other
command SHALL leave the command line open with `E: not a command` in the label.

The text SHALL be capped, and a key that would grow it past the cap SHALL do nothing
visible rather than drop characters. A line longer than the bar SHALL scroll
sideways under the caret.

#### Scenario: A multi-line question is written and sent

- **WHEN** `Ctrl+G` is pressed, four lines are typed with `Enter` between them, and `:wq` is pressed
- **THEN** the bar returns to one row holding that text, and `Enter` sends a question whose body is those four lines

#### Scenario: The editor gives the text back on the way out

- **WHEN** the editor is left with `Esc` in normal mode or with `Ctrl+G`
- **THEN** what was typed is in the input line, unchanged

#### Scenario: Nothing is kept when it is thrown away

- **WHEN** `:q!` is used to leave
- **THEN** the input line holds exactly what it held before `Ctrl+G`

#### Scenario: The panel comes back to its shape

- **WHEN** the editor is opened and then left
- **THEN** the surface is taller only while the editor is open, and its bottom edge never moves

#### Scenario: The replies are untouched

- **WHEN** the editor is open over an answer
- **THEN** the replies card is the same size and shows the same rows, and only the bar's height changed

#### Scenario: A command that is not one

- **WHEN** `:w` is typed and `Enter` is pressed
- **THEN** the command line stays open, the label says `E: not a command`, and the editor is still there
