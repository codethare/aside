# Proposal

## Why

Every answer of the run stays on screen and scrolls in the box, so a follow-up is
read at the bottom of everything asked before it. The panel is not a transcript
viewer: what matters is the answer to the question just asked, and the older ones
only matter to the provider.

## What Changes

- The replies box shows **only the current turn**: the answer, failure or note of
  the question just asked. A previous answer, a previous failure and a previous
  note are not drawn, take no room, and do not scroll.
- The transcript still keeps every turn for the request: a follow-up carries the
  earlier questions and answers exactly as before, and a cancelled or cut answer is
  still left out of it.
- Sending a question therefore clears the box (the ball is up while the request
  runs) and it fills with the new answer alone.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the transcript draws the current turn only; earlier turns are kept
  for the request alone.

## Impact

- `src/transcript.rs`: a turn that is no longer current is laid out never, like a
  question already is.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`.
- No new dependency, no new file, nothing on disk.
