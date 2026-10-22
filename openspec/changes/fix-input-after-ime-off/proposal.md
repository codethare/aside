# Proposal

## Why

After Chinese has been typed once, fcitx switching off — or quitting — leaves the
panel unable to type English or digits at all. The panel latches "an input method
exists" on the first event it ever receives from one and never looks at a keysym
again, so once the input method stops delivering, nothing does.

## What Changes

- The panel decides **per keystroke**, not once and for all: a text key pressed
  while an input method is in play is held for the rest of the current event round.
  If the input method speaks in that round — a pre-edit, a commit, a deletion or a
  done — the held key was its business and is dropped. If it stays silent, the held
  key's text is inserted, so English and digits keep working with the input method
  switched off.
- A key that arrives while a pre-edit is open still types nothing, as now.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: "the input method takes over" becomes a per-keystroke arbitration
  instead of a permanent latch.

## Impact

- `src/ime.rs`: a held-key buffer and the arbitration.
- `src/main.rs`: text keys held instead of inserted, flushed at the end of the
  round, dropped when the input method speaks.
- `README.md` (the IME paragraph, if it needs it).
- No new dependency, no new file, nothing on disk.
