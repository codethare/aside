# Proposal

## Why

Two defects in the panel as it stands. The replies card is as tall as the room
allows but the lines inside it are still placed from the transcript's own origin,
so as soon as the answer is taller than the card every visible line is drawn below
it and nothing shows. And the surface is anchored only to the bottom edge, where a
layer-shell margin has no effect horizontally: dragging and `h`/`l` can only move
the panel up and down, and moving it up and down changes the gap the height cap is
derived from, so the panel resizes while it is being moved.

## What Changes

- The replies card is a **fixed size** for as long as the answering panel is up:
  the bar, the 12 px between the cards, and the cap derived from the **anchored**
  gap. It no longer grows with the content; a longer answer scrolls inside it, and
  moving the panel no longer resizes it.
- Lines are placed by the scroll offset, so the newest rows are visible when the
  answer is taller than the card.
- The surface is anchored to the **left** edge as well as the bottom, so the left
  margin is a real position: dragging and `h`/`l` move the panel in both axes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the answering panel has a fixed height; its cap comes from the
  anchored gap and not from the drag; a move is a move, not a resize; the panel
  moves freely on both axes.

## Impact

- `src/main.rs`: the anchor, `size_for` losing its content and drag arguments.
- `src/panel.rs`: the card's height and the rows' offset.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`.
- No new dependency, no new file, nothing on disk.
