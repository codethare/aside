# Design

## Context

Vim mode's `hjkl` move the highlight (`vim_move`), and the panel moves only by
dragging, through `drag_to` → `apply_size`. The drag clamps the left margin but not
the bottom one: nothing stops a drag from pushing the surface off the top.

## Goals / Non-Goals

- One clamped move used by both the pointer and the keyboard.
- Vim-idiomatic keys for what `hjkl` gives up.

**Non-Goals:**

- Subpixel or animated movement, momentum, snap-to-edge.
- Remembering the position (nothing on disk).
- Moving the panel from typing mode: it takes vim mode or the pointer.

## Decisions

**1. One `move_panel(dx, dy)` behind both inputs.** It applies the delta to the
drag offset, clamps the resulting *margins* to the output, and converts the
clamped margin back into an offset, so `edge_gap` — and with it the height cap —
always equals the gap the user can see. `drag_to` and `window_move` both call it,
and neither owns geometry of its own.
*Alternative rejected:* clamping only in `margins()`, which leaves the drag offset
disagreeing with the painted position and the cap derived from a gap that is not
there.

**2. `hjkl` steps the window; the transcript moves to `n`/`p`, `Ctrl+n`/`Ctrl+p`,
`g`/`G`.** The user asked for the spatial keys to be spatial. `n`/`p` already
paged; the line step and the ends move to their vim-idiomatic keys, so nothing the
mode could do before is lost.
*Alternative rejected:* a sub-mode toggle for the window (`w`), which would make
`hjkl` mean two things depending on invisible state.

**3. Steps are 16 px, ten times that with `Ctrl`.** 16 px is the panel's own
padding: a step is one visible unit at every scale, and ten of them is a screen's
worth of travel in a couple of presses. Both are constants next to the reveal's.

## Risks / Trade-offs

- [A step re-commits the surface like a drag frame] → one commit per keypress, and
  no re-wrap: the transcript is laid out for the width, which does not change.
- [Moving up grows the gap, so the cap can exceed the room above] → `reply_cap`
  clamps by the room above the bar, so the panel still fits; the test at 1080 with
  a 1010 px gap pins that.
- [A press against an edge is a no-op that could look like a lost keystroke] → the
  panel simply does not move, which is what every window manager does at an edge.
