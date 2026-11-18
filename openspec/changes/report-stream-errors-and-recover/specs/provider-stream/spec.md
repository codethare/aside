## ADDED Requirements

### Requirement: An error sent inside the stream is a failure

A `data:` frame carrying the provider's `{"error": ...}` object SHALL be reported as
the provider's failure, with its `message`, not as a frame without text. It SHALL
outrank a later `[DONE]`, and text that arrived before it SHALL stay on screen. An
error object with no message SHALL still fail, with a line that says the provider
reported an error rather than inventing a cause.

#### Scenario: An error frame after some text

- **WHEN** the stream carries two content deltas and then `data: {"error":{"message":"rate limited"}}`, and the connection closes
- **THEN** the transcript keeps the text that arrived and adds one failure entry containing `rate limited`

#### Scenario: An error frame before the sentinel

- **WHEN** the stream carries `data: {"error":{"message":"context too long"}}` and then `data: [DONE]`
- **THEN** the request ends as a failure naming `context too long`, not as a complete empty answer

#### Scenario: A content string that mentions error

- **WHEN** a content delta's own text contains the word `error`
- **THEN** it is still answer text, because only a payload that is the provider's error object is an error
