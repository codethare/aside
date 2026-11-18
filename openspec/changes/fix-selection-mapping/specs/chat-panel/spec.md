## MODIFIED Requirements

### Requirement: Reply lines can be selected with the pointer

A press of the left button on a reply line SHALL anchor a selection at the
**character** under the pointer; motion with the button down SHALL extend it to the
character under the pointer, releasing SHALL leave it in place, and pressing again
SHALL replace it. The selection SHALL be drawn as a band behind exactly the
characters it covers — the rows between its ends whole, the first and last cut where
the pointer named — with the text in the primary ink on top of it, and SHALL NOT
move any text. A press that is not on a reply line SHALL NOT start a selection, and
SHALL move the panel instead if it moves.

The character under the pointer SHALL be computed in the same coordinates the band
is drawn in: the pointer's surface position times the buffer scale, less the card's
inset (shadow plus padding), less the row's own indent, less the keyboard
highlight's two-pixel nudge when that row is drawn inked. A character index SHALL
mean a character, not a byte, everywhere a selection and a copy use it. The whole
replies card, down to its last row, SHALL be selectable.

`Alt+C` and `Ctrl+C` SHALL copy the selected characters: the first and last lines
in part, the lines between whole, one line per line. `Esc` SHALL clear the selection before it cancels a stream or closes the
panel, sending a question SHALL clear it, and a selection MUST NOT survive a turn
change: the lines it named are no longer on screen. A re-wrap — a width or scale
change — SHALL also clear it, because the row and character indices no longer name
the same text. A copy that selected nothing SHALL NOT be reported as a copy.

#### Scenario: Selecting and copying three lines

- **WHEN** the user presses on the first line of an answer, drags to its third line, releases, and presses `Ctrl+C`
- **THEN** the clipboard holds those three lines, one per line, and nothing else

#### Scenario: Passing over the text selects nothing

- **WHEN** the pointer moves across the replies with no button held, after a selection has been made and released
- **THEN** the selection is unchanged, and no text is banded by the movement

#### Scenario: A selection can stop mid-word

- **WHEN** the user drags from the fourth word of a line to the second word of the next
- **THEN** only those characters are banded and copied, and the rest of each line is not

#### Scenario: A multibyte character is one index

- **WHEN** the pointer presses after one CJK character
- **THEN** the anchor is character 1, the band starts there, and the copy slices at that character's byte offset

#### Scenario: The scale does not shift the character

- **WHEN** the panel is drawn at 2x and the pointer presses over a character
- **THEN** the character under it is the one the band starts on, because the pointer's logical position is scaled with the text

#### Scenario: An indented line maps through its indent

- **WHEN** the pointer presses over a list item or a code row that is indented
- **THEN** the character under it accounts for that row's indent, not the card's inset alone

#### Scenario: The last rows of the card select

- **WHEN** the pointer presses on the replies card's last visible row, just above the gap to the bar
- **THEN** a selection starts on that row, and the press does not drag the panel

#### Scenario: Escape clears before it closes

- **WHEN** a selection exists and the user presses `Esc` with no stream open
- **THEN** the selection is cleared and the panel stays open; a second `Esc` closes it

#### Scenario: A selection cannot outlive its turn

- **WHEN** the user sends a question with a selection on screen
- **THEN** the selection is gone, and `Ctrl+C` copies the newly highlighted line only

#### Scenario: A resize invalidates the indices

- **WHEN** a selection exists and the panel's width or scale changes
- **THEN** the selection is cleared rather than copying characters from the new wrap
