# Tasks

## 1. The reader knows an error frame

- [x] 1.1 `sse::Event::Error(String)`; a `data:` payload that is the provider's
  `{"error": ...}` object yields the error's `message` — verify: a test feeds an error
  frame and gets one `Error`, a content frame whose text contains the word `error`
  still yields its `Delta`, and the existing frame tests are unchanged
- [x] 1.2 A frame without a message is still an error, with a generic line — verify: a
  test feeds `data: {"error":{}}` and gets an `Error` whose text is not empty

## 2. The request reports what happened

- [x] 2.1 `Request` carries the first in-stream error and `finish` returns
  `Close::Failed` naming the provider's message, even when `[DONE]` follows — verify:
  a scripted request test sends content, an error frame and `[DONE]`, and asserts
  `Close::Failed` carrying `rate limited` while the text stays in the transcript
- [x] 2.2 A complete stream with no text yields `Close::Empty`; the panel closes the
  empty answer and adds `the provider sent no answer text` — verify: a scripted
  request test with only a role frame and `[DONE]` asserts `Close::Empty`, and a
  transcript test asserts an empty complete answer completes no turn
- [x] 2.3 Deltas drained in one tick are appended together, so a burst costs one
  re-layout — verify: `poll` drains the reader into one `String` before calling
  `transcript.delta`, and the scripted multi-frame test holds the frames in order

## 3. The question comes back

- [x] 3.1 On `Close::Failed`, if the input line is empty, move the bar's question back
  into it and clear the bar's copy — verify: the close path is scripted in the chat
  tests; the input-line branch is exercised by live checklist 16b, which has no
  headless equivalent
- [x] 3.2 If the reader has typed since sending, their text stays and the question
  leaves the bar — verify: the same input-line branch guards on `is_blank`, and 16b
  checks it in a session
- [x] 3.3 `request_turns` pairs a question with a complete, non-empty answer and drops
  an unanswered or failed question pair from later requests — verify: transcript tests
  cover a failed turn followed by a new question, a cancelled turn, an empty answer,
  and the newest open question still being sent alone

## 4. Integration

- [x] 4.1 Run the gate — verify: 255 tests pass and the four pre-existing
  font-environment failures are the only failures; `cargo fmt --check` and `cargo
  clippy --all-targets -- -D warnings` are clean
- [x] 4.2 Update the specs and the live checklist for the new failure and recovery
  wording — verify: the deltas parse with `openspec validate
  report-stream-errors-and-recover --strict`, and checklist 16b names the retry
- [ ] 4.3 Run a real session — verify: checklist 16b on a failing endpoint, and an
  in-stream provider error (if one can be provoked) shows the provider's message
