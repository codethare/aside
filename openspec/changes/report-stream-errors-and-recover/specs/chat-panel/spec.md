## MODIFIED Requirements

### Requirement: Turns accumulate in one transcript

The panel SHALL keep every turn of the current run in one ordered transcript, and
sending a question MUST keep all earlier replies available to the request. Each
request MUST carry the preceding turns of this run in order, and MUST NOT carry
anything from a previous run of the process. An answer that was cancelled or cut
short SHALL be left out of subsequent requests rather than sent back to the
provider as if it were complete. A question whose answer never completed SHALL be
left out of subsequent requests too: the panel pairs a question with a complete,
non-empty answer, and the newest question is sent on its own while its answer is
open. A question the reader sends again therefore appears once, not twice.

#### Scenario: A follow-up keeps the first answer

- **WHEN** the user asks a question, receives an answer, and asks a follow-up
- **THEN** the request body contains the first question, the first answer and the follow-up, even though only the last answer is on screen

#### Scenario: Nothing survives a restart

- **WHEN** `aside` is closed and started again
- **THEN** the transcript is empty and the request contains only the new question

#### Scenario: A failed turn is not carried

- **WHEN** one question ends in a failure and the user sends another question
- **THEN** the request carries only the new question, not the failed question or its partial answer

#### Scenario: A cancelled turn is not carried

- **WHEN** an answer is cancelled part way through and the user sends a follow-up
- **THEN** the request carries the earlier complete turns, the follow-up, and neither the cancelled answer nor its question

### Requirement: Failures and empty states are part of the transcript

A provider or transport failure SHALL be rendered as its own transcript entry
distinct from answer text, naming the provider and the cause, and the panel MUST
remain interactive and closable in every failure state. An empty transcript MUST
be distinguishable from a pending one.

An answer that ended without any text SHALL NOT be left as a blank replies card:
the panel SHALL say the provider sent no answer text, and the turn SHALL be treated
as unanswered.

#### Scenario: Rejected credential

- **WHEN** the provider answers `401`
- **THEN** the transcript shows one entry naming the provider and the environment variable that should hold its key, and `Esc` still closes the panel

#### Scenario: Empty state

- **WHEN** the panel opens and nothing has been asked yet
- **THEN** the input line is focused and the transcript area is drawn as an empty state, not as a blank pending answer

#### Scenario: An answer with no text

- **WHEN** the provider ends the stream with `[DONE]` having sent no content
- **THEN** the replies card holds one note saying no answer text arrived, and the next request does not carry that empty turn

## ADDED Requirements

### Requirement: A failed question goes back to the line

When a request fails and the input line is still empty, the question that was sent
SHALL be placed back in the input line, with the caret at its end, ready for
`Enter`. If the reader has typed since sending, their text SHALL stay and the
question SHALL leave the bar as it does on a completed answer. A cancelled or
truncated answer SHALL NOT restore anything: its partial text is the answer the
reader asked for.

#### Scenario: A failed request returns the question

- **WHEN** a question is sent and the request fails before any answer text, and the input line is empty
- **THEN** the input line holds the question again, and pressing `Enter` retries it

#### Scenario: Typed text outranks the restored question

- **WHEN** the request fails after the reader has started typing a new question
- **THEN** the input line keeps what they typed, and the failed question is not inserted
