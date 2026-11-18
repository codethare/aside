# Tasks

## 1. The request history is bounded

- [x] 1.1 `CONTEXT_CAP` (512 KiB), `enforce_context_cap(cap)`: drop the oldest turns'
  text until the total fits, never the newest question or the turn in flight — verify:
  a test with six 100-byte turns and a 250-byte cap asserts the request's text fits,
  the newest question survives, the oldest turn is gone and its text is freed
- [x] 1.2 Run it in `send` before the body is built; show one note after the exchange
  ends (or immediately if the request never starts), and on cancel — verify: a test
  pins the newest question under a zero cap; the note is appended after the turn is
  closed by every ending, and on cancel, so it cannot disturb the open answer's layout
- [x] 1.3 The note is visible after complete, failed, empty and cancelled turns —
  verify: the note is a transcript `Note` entry appended after the closing entry, and
  the code path is shared by all four endings; the live check is 4.2

## 2. The response queue is bounded

- [x] 2.1 `Feed` has a 1 MiB cap; its reader waits for room and resumes when `take`
  drains; `close` wakes it and `join` waits — verify: a test reads an endless source,
  sees the queue stop exactly at `FEED_CAP`, then closes and joins
- [x] 2.2 `Request::finish` closes and joins before reaping, then decodes the tail the
  reader queued after the last `poll`; `cancel` and `Drop` do the same — verify: the
  scripted request tests still end and decode, and the live/Frames tests do not hang

## 3. A capped answer stops costing layout

- [x] 3.1 `Transcript::delta` returns whether a row changed; after `REPLY_CAP` it is
  false and `poll` reports no change — verify: the reply-cap test asserts a delta after
  the cap returns false and the rows did not grow

## 4. Integration

- [x] 4.1 Run the gate — verify: 262 tests pass, only the four pre-existing
  font-environment failures fail, `cargo fmt --check` and `cargo clippy --all-targets
  -- -D warnings` are clean
- [ ] 4.2 A session past 512 KiB of turns shows the note and still answers — verify:
  live check, not reproducible headlessly without a long exchange
