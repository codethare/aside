# Proposal

## Why

Two defects of the fixed box. Asking a follow-up folds the panel shut and opens it
again — the shape falls back to the bar while the request is out, then grows when
the answer starts — so a second question reads as the box shrinking twice. And a
scrolled view draws the row that is half above the card, whose descenders land on
the row below: the two topmost lines squash together.

## What Changes

- **A request in flight keeps the box it was asked from.** Asking a follow-up leaves
  the answering panel exactly as it is, with the working light on the bar's rim;
  only the first question of a run grows the panel out of the bar.
- **A row that starts above the card is not drawn.** The top row is dropped whole
  while it is partly scrolled out, so no ink from it can reach the row below.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the thinking state's shape depends on where the panel already is;
  the replies card clips a partly scrolled row rather than drawing it.

## Impact

- `src/main.rs`: the shape a phase is drawn as.
- `src/panel.rs`: the row clip.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`.
