# Proposal

## Why

Scrolling over the input bar scrolls the replies, which the spec says it must not.

`over_replies` measures the bar's top as `SHADOW + (panel_h - bar_height)`, but the
bar hangs from the panel's bottom edge, which is `SHADOW` in from the surface's: its
top is `panel_h - SHADOW - bar_height`. The check is therefore `SHADOW` too generous,
and at the default size that covers the 16 px gap between the cards, the whole bar,
and 10 px of the shadow margin below it — the entire input bar takes a scroll.

The reader sees the consequence, not the arithmetic: they rest the pointer on the bar
while an answer streams, flick the wheel (which should do nothing), and the view
scrolls up and stops following. Once the answer ends the view is stuck above the last
line, because the only thing that re-pins it is the request finishing, and the scroll
came after that.

## What Changes

- **The bar and the gap above it take no scroll.** `over_replies` ends at the replies
  card's own bottom edge, which is where the bar's gap begins. Scrolling there does
  nothing, as the spec already says; only the replies card scrolls.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the replies card is the only region that scrolls; the gap between the
  two cards and the bar below it do not, and that boundary is the card's own bottom
  edge rather than an estimate of where the bar starts.

## Impact

- `src/main.rs`: `over_replies`, and the test that pins the region.
- No new dependency, nothing on disk, no layout change: the card, the gap and the bar
  are drawn where they were.
