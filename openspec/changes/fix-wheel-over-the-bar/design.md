# Design

## Context

`panel::draw` puts the bar on the panel's bottom row — `panel.y + panel.h - bar_h`,
with `panel` inset by `SHADOW` — and the replies card in the room between the panel's
top and that bar, less `SEPARATION`. `over_replies` is the pointer's sole idea of
which region the wheel belongs to; `transcript_y` and `reply_spot` already measure
from the card's top correctly, and count only the bar differently.

## Goals / Non-Goals

- One region scrolls, and it is the region the replies are drawn in.
- Non-Goals: changing where the cards are drawn, the selection's own hit test, or the
  follow rule — an explicit scroll on the replies still stops following, and the end
  of a request still re-pins the view.

## Decisions

**1. The boundary is the replies card's bottom edge, computed from the same numbers
`draw` uses.** `panel_h - SHADOW - bar_height - SEPARATION`, compared in logical px
like the pointer's y. Deriving it from `bar_height()` and `SHADOW` rather than
hard-coding 344 px is what keeps it true when the face size or the box size changes.
*Alternative rejected:* keeping the estimate and subtracting `SHADOW` twice — it
reads as a fudge, and the next reader cannot tell which inset it is correcting.

**2. The gap between the cards is not the card.** A pointer sitting in the 16 px
between the two cards is over the panel's background, not over a row of text, so a
wheel there does nothing. This is the same answer the shadow margin gets.
*Alternative rejected:* letting the gap scroll (it is "between" the cards, so it
belongs to no card) — then a pointer resting one pixel off the card scrolls, which is
the class of surprise this change is about.

**3. The rule that re-pins the view stays where it is.** An answer's last text
follows the request's end, so a view scrolled up during the stream returns to the
bottom there. Nothing here changes that: the reported stuck view came from a scroll
that landed *after* the last text, on a bar that should not have scrolled at all.
