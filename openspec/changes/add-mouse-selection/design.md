# Design

## Context

`pointer_frame` records a press time, and `check_hold` starts the drag once
`DRAG_HOLD_MS` has elapsed — which is why the panel starts moving 250 ms after the
cursor does. `draw`/`draw_replies` take `highlight: Option<usize>`, one row index,
and `copy_highlight` copies that one row. The transcript can answer `row_y(index)`
but nothing maps a y back to a row.

## Goals / Non-Goals

- Pointer drags that start at once; a line-range selection with a pointer and a
  copy that takes it.

**Non-Goals:**

- Character-precise selection and the drawing that goes with it (a per-glyph
  extent for every line, every frame), double-click word selection, or a selection
  that spans a scroll while dragging.
- Copying with the primary selection on release (X11-style): the copy stays a
  deliberate key, as the spec has always required.

## Decisions

**1. The write gesture is distinguished by where it starts, not by a timer.** A
press on a reply line starts a selection; a press anywhere else on the panel starts
a drag. The first motion decides which one runs, so neither waits and a click does
nothing.
*Alternative rejected:* keeping the hold and starting the drag from the press
position when it elapses — the panel would jump 250 ms of pointer travel, which is
the same lag seen from the other side.

**2. The selection is a pair of row indices, and it reuses the highlight's ink.**
`Option<(usize, usize)>` in `State`, normalised so either end can be the anchor;
painting asks "is this row in the range", which is the same mark the keyboard
highlight already draws. No new colour, no new geometry.
*Alternative rejected:* absolute pixel rectangles — they would have to be
recomputed on every scroll and would not survive a re-wrap.

**3. `Transcript::row_at(y, blocks)` maps a pointer y to a row.** The pointer's y
is turned into a transcript offset by the card's origin and `viewport.top`, then
the blocks are walked — the same walk `row_y` does the other way, and O(rows) only
on a pointer event.
*Alternative rejected:* keeping a hit-test table per frame — one more thing to keep
in step with the layout.

**4. Row ranges, not glyph ranges, because the panel copies lines.** `Alt+C` has
always meant "this line"; a selection that copied half a line would make the key
mean two things depending on how it was reached.
