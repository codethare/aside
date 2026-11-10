# Spec Delta

## ADDED Requirements

### Requirement: The question stays in the bar while it is out

After `Return` the bar SHALL keep showing the question that was sent, for as long as
the request it started is unfinished. The line SHALL NOT be editable while it shows a
question: no caret and no pre-edit SHALL be drawn, and no key SHALL reach the text.

A question wider than the bar SHALL scroll from right to left at a slow, steady speed;
on reaching its end it SHALL hold, reverse, and hold again at its start, turning round
for as long as the request is unfinished. A question that fits SHALL NOT move.

The question SHALL leave the bar when the request ends — a complete answer, a
truncated answer, a failure, or a cancel — and when the reader writes into the line,
which starts a fresh question. The transcript SHALL still draw no question, and the
question SHALL NOT be written anywhere.

#### Scenario: A long question is sent

- **WHEN** a question wider than the bar is sent and the model is still working
- **THEN** the bar still shows it, scrolling from its first character to its last and
  then back, and no caret is drawn

#### Scenario: A short question is sent

- **WHEN** a question that fits in the bar is sent
- **THEN** it stays in the bar and does not move

#### Scenario: The request ends

- **WHEN** the answer finishes, fails, is truncated, or is cancelled
- **THEN** the bar is empty again, showing no question and a caret if it has the keyboard

#### Scenario: A follow-up is begun while the answer streams

- **WHEN** the reader types into the bar while a request is still running
- **THEN** the question leaves the bar and the typing is what the line shows

#### Scenario: Copy mode takes the bar

- **WHEN** copy mode is entered while a question is still out
- **THEN** the bar shows the copy label, not the question, and shows the question again
  when copy mode is left
