# Proposal

## Why

Two defects of the pointer, both about it not doing what a pointer does. Dragging
the panel waits a quarter of a second before it starts, so the cursor moves and the
panel follows late. And text cannot be selected at all: the only way to take more
than one line out is to move the keyboard highlight line by line.

## What Changes

- **The panel follows the pointer immediately.** A press that moves carries the
  panel from the first motion event; a press that does not move does nothing. The
  250 ms hold, and the lag it caused, are gone.
- **The replies can be selected with the pointer.** A press on a reply line anchors
  a selection, motion extends it over whole lines, release leaves it in place, and
  the selected lines are drawn in the highlight ink. `Alt+C` and `Ctrl+C` copy
  every selected line, one per line, or the single highlighted line when nothing is
  selected; `Esc` and sending a question clear it.
- A press on the bar, or on the replies card away from its text, still moves the
  panel, so the two gestures do not compete.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: a drag starts on motion rather than after a hold; the pointer can
  select reply lines; copying takes the selection.

## Impact

- `src/main.rs`: the press/motion/release handling and the selection state.
- `src/panel.rs`: rows inside the selection are painted highlighted.
- `src/transcript.rs`: a row hit-test for the pointer.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`.
- No new dependency, no new file, nothing on disk.
