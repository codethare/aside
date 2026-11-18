# Proposal

## Why

Three failure paths currently tell the reader the wrong thing.

A provider that reports an error inside the stream — `data: {"error": ...}` after a
200 — is not a frame this reader knows. The error text is dropped, and the panel
reports either "the answer stopped arriving part way through" or, if `[DONE]`
follows, shows an empty replies card with no explanation.

A provider that ends with `[DONE]` and no text at all is treated as a complete
answer, so the panel grows its fixed replies card around nothing.

A request that fails closes its question out of the bar and leaves it in the
transcript only as context. The line the reader typed is gone, although they have to
send it again; and because the unanswered question stays in the request history,
sending it again puts it in front of the model twice.

## What Changes

- **An in-stream error is a failure.** `sse::Event::Error` carries the provider's
  `message`; the request reports it as a failure, with any text that arrived before
  it kept on screen.
- **An empty answer says so.** A stream that ends complete with no text closes the
  empty answer and adds a note, rather than leaving a blank card.
- **A failed question returns to the input line** when the reader has not typed
  anything else. If they have, their text wins and the question leaves the bar as
  before.
- **Only answered turns are sent back.** A question whose answer never completed is
  dropped from later requests; the newest question is still sent, and a question the
  reader sends again appears once, not twice.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `provider-stream`: the reader recognises an error frame and the request reports it
  as the provider's failure, not as a stalled or empty stream.
- `chat-panel`: an empty answer is a visible note; a failed question goes back to the
  input line; a turn that never got an answer is not carried in the next request.

## Impact

- `src/sse.rs`: `Event::Error` and the error-frame reader.
- `src/chat.rs`: coalesced deltas, the in-stream failure, the empty-answer outcome.
- `src/transcript.rs`: question/answer pairing in `request_turns`.
- `src/main.rs`: closing the request, restoring the question on failure.
- No new dependency, nothing on disk.
