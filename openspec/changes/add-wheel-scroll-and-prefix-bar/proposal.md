# Proposal

## Why

The transcript can only be scrolled from the keyboard, and the prefix mode is
invisible: after `Ctrl+T` the bar looks exactly as it did, so the next keystroke
disappearing is a surprise.

## What Changes

- **The wheel scrolls the replies.** While the pointer is over the replies card, a
  wheel notch or a two-finger touchpad slide moves the transcript; a notch is three
  lines, a touchpad's pixel delta is taken as pixels. Reaching the bottom resumes
  following the newest line; scrolling up stops it.
- **The prefix mode shows itself.** While `Ctrl+T` waits for its one key, the bar
  cannot take text, its body takes a mode tone, and it reads `press [ to enter vim
  mode` instead of the input line, with no caret.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the pointer scrolls the replies; the prefix mode is visible in the
  bar's tone and text, and the bar takes no input while it waits.

## Impact

- `src/main.rs`: the axis events, the pointer's position against the replies card,
  and the bar's hint and tone.
- `src/layout.rs`: `Viewport::scroll_by`.
- `src/panel.rs`: a per-card body tone and a hint string in place of the input.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`.
- No new dependency, no new file, nothing on disk.
