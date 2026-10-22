# Spec Delta

## MODIFIED Requirements

### Requirement: The transcript draws only the current turn

The replies box SHALL show what the model or the panel said about **the question
just asked**: its answer, and the failures or notes that belong to it. It SHALL NOT
draw the answers, failures or notes of earlier turns, which SHALL take no rows, no
height and no scroll. The transcript SHALL still keep every turn of the run in
order for the request that follows, and a question SHALL still be kept and drawn
nowhere. `Up`, `Down`, `PageUp`, `PageDown` and `Alt+C` SHALL address the lines on
screen, so they never reach an earlier turn's text.

#### Scenario: A follow-up shows its own answer alone

- **WHEN** the first answer is on screen and the user asks a follow-up
- **THEN** the box holds the new answer only, and the first answer is not on screen

#### Scenario: The follow-up still carries the history

- **WHEN** the second answer is requested
- **THEN** the request carries the first question, its answer and the second question, in order

#### Scenario: A failure belongs to its turn

- **WHEN** a turn ends in a failure or is cancelled, and a new question is then sent
- **THEN** the failure or the note is not on screen after the new question, and the box holds the new turn alone

#### Scenario: Nothing reaches back

- **WHEN** the user copies the highlighted line after several turns
- **THEN** the line is one of the current turn's lines

### Requirement: Turns accumulate in one transcript

The panel SHALL keep every turn of the current run in one ordered transcript, and
sending a question MUST keep all earlier replies available to the request. Each
request MUST carry the preceding turns of this run in order, and MUST NOT carry
anything from a previous run of the process. An answer that was cancelled or cut
short SHALL be left out of subsequent requests rather than sent back to the
provider as if it were complete.

#### Scenario: A follow-up keeps the first answer

- **WHEN** the user asks a question, receives an answer, and asks a follow-up
- **THEN** the request body contains the first question, the first answer and the follow-up, even though only the last answer is on screen

#### Scenario: Nothing survives a restart

- **WHEN** `aside` is closed and started again
- **THEN** the transcript is empty and the request contains only the new question
