# Proposal

## Why

The layer surface is larger than the panel: it carries a 26 px shadow margin on every
side, and the 16 px gap between the replies card and the bar. Wayland's default input
region is the whole surface, so clicks in that transparent border and gap never reach
the window visible underneath. The panel receives them instead, which looks like a
window that cannot be clicked: the compositor's active window may already be the one
under the pointer, while the keyboard stays with `aside`.

## What Changes

- Set the surface's input region to the two cards and nothing else: the replies card
  and the bar. The shadow margin and the gap between them pass pointer input through
  to whatever is below, so clicking a window seen there focuses and types into it
  normally.
- Recompute the region whenever the panel's size changes (a state change or a resize),
  because the cards move with it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: only the two cards take pointer input; the chrome's transparent
  margin and the gap are click-through.

## Impact

- `src/main.rs`: a region per size, set before each size is committed.
- No dependency, nothing on disk.
