# Proposal

## Why

Three queues still grow with the session rather than with what is on screen.

A run's request carries every completed turn. Each answer is capped at 64 KiB but
the number of turns is not, so a long-lived panel builds a request body of megabytes
(a hundred turns is roughly 12.5 MiB), keeps the same text in memory, and pays the
provider for it on every follow-up until the provider refuses the request.

The background reader queues bytes without a bound. The panel drains it every 16 ms,
which is normally enough, but a slow frame or a stalled loop lets a fast stream grow
the queue until the reply cap is reached and beyond.

Once an answer reaches `REPLY_CAP`, every further delta is decoded, and `poll`
reports a change, so the panel keeps laying out and repainting an answer that cannot
grow.

## What Changes

- **The request carries at most 512 KiB of turn text.** When a new question would
  exceed it, the oldest turns are dropped from the context and their text is freed;
  the newest question and the turn in flight are never touched. The transcript says
  so once the exchange ends, rather than silently forgetting.
- **The reader stops when 1 MiB is queued**, and resumes when the UI drains. When the
  request ends or is cancelled, `finish`/`cancel` close and join the reader, then take
  whatever it queued after the last poll, so a child that exits between two polls
  does not lose its tail.
- **A capped reply costs no layout.** `Transcript::delta` returns whether a row
  changed; after the cap it returns false, and `poll` no longer asks for a frame. The
  stream is still read to `[DONE]`, because the ending must be known.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the request history is bounded and drops the oldest turns with a
  visible note; a capped answer stops laying out.
- `provider-stream`: the response queue is bounded, and the request's end drains
  what the reader already queued.

## Impact

- `src/transcript.rs`: `CONTEXT_CAP`, `enforce_context_cap`, `delta` returns a bool.
- `src/feed.rs`: a bounded queue with backpressure, `close`/`join`.
- `src/chat.rs`: `poll`/`finish` share a decode step; `finish` drains and joins.
- `src/main.rs`: the cap runs before the body is built; the note is shown after the
  turn, and on cancel.
- No new dependency, nothing on disk.
