# Design

## Context

`size_for` sizes the answering panel as `bar + 12 + min(content, cap)`, and
`panel::draw` gives the replies card `min(room, transcript total)`. The lines
inside are drawn at `card.y + row.y`, where `row.y` is measured from the
transcript's own origin, while the viewport's `top` is only used to decide *which*
lines to draw — so once `top > 0` every drawn line lands below the card and
nothing is visible. The anchor is `Anchor::BOTTOM` alone, and a layer-shell margin
applies only to an anchored edge, so the left margin is a no-op and only vertical
movement is possible.

## Goals / Non-Goals

- A fixed answering box, lines that scroll inside it, and movement on both axes.

**Non-Goals:**

- Remembering the position, animating movement, changing the cap's 1× share.
- Growing the box with the content in any state.

## Decisions

**1. The box keeps its height; the lines keep their offset.** The card's height is
always the room, and each line is drawn at `card.y + row.y - viewport.top`. The
`min(content, room)` and the missing subtraction were the same bug seen twice: the
card was a clipped window while the lines were placed as if it were the whole
transcript.
*Alternative rejected:* keeping the growth and only fixing the offset — the user
asked for a fixed box, and a box that changes height while it is being dragged is
what made the defect hard to see.

**2. `cap` stops depending on the drag.** It comes from the anchored gap, so moving
the panel cannot resize it. The drag still moves the margins, and
`drag_for_bottom` keeps the offset equal to the gap on screen, but the cap ignores
the offset.
*Alternative rejected:* a cap that follows the drag — that is the resize the user
reported.

**3. `Anchor::BOTTOM | Anchor::LEFT`.** Two anchors, so the compositor honours the
left margin; without `Anchor::RIGHT` the surface keeps its own width, and
`left_margin` still centres it by default. One line, and both the drag and `h`/`l`
start working.
*Alternative rejected:* anchoring left and right and letting the compositor
stretch the width — the panel's width is fixed by design.

## Risks / Trade-offs

- [A fixed box is tall even for a one-line answer] → that is the request; the
  reveal still makes the card rise out of the bar rather than appear whole.
- [Moving the panel near an edge then resizing the output (scale change) leaves the
  drag offset outside the new bounds] → every margin goes through the clamps on
  each `apply_size`, so the panel is pulled back on screen.
- [`MAX_ROWS` now bounds the box height as well as the viewport] → the box is
  `min(cap, 64 rows)`, which keeps the frame cost per visible row.
