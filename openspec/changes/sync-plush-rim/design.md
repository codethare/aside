# Design

## Context

`aside`'s `render.rs` was ported from `wlmenu`, which was aligned to `plush` at
08:52 today: `SHADOW` 20, layers `(6, 155, 1)`/`(13, 115, 4)`, lit ink alpha 34,
dark underside 60, two-stop-count profiles. `plush` itself moved on at 11:49 with
`Light the panel rim like glass and widen the shadow`.

## Goals / Non-Goals

- The numbers and the anatomy of `plush`'s current chrome, in `aside`'s row-major
  painter.

**Non-Goals:**

- `plush`'s stroke-width configuration (it has a `[border] width` key; `aside` has
  no config surface and stays at one pixel), its `LinearGradient` machinery, its
  stacked panels, or its animation.
- Re-aligning `wlmenu`, which lives in its own repository.

## Decisions

**1. The three strokes become, per row, a rim pixel and a band pixel.** `plush`
strokes three rounded outlines inset by 0, 1 and 2 px with per-stroke gradient
profiles. In a row-major painter the same thing is: the outermost pixel of the span
gets the dark underside then the lit rim, and the pixel one inside gets the band;
on the two rows where the band's horizontal edges lie (one below the top edge, one
above the bottom edge) the band runs across the whole inner span.
*Alternative rejected:* stroking a second rounded outline — `aside` has no path
stroke, and `rounded_span` already gives the outline row by row.

**2. `SEPARATION` goes to 16 with `plush`'s own compile-time proof.** `plush` asserts
`GAP > SHADOW_REACH_UP`, where the reach is the wide layer's blur less its downward
offset (20 − 6 = 14). `aside` gets the same constant and the same assertion, so the
number cannot be lowered past the shadow's reach again without the build failing.

## Risks / Trade-offs

- [`wlmenu` is still on the old numbers, so the two products differ until it is
  re-synced] → the divergence is named here and in the commit; `plush` is the source
  both copy from.
- [A 26 px margin makes the surface 12 px bigger on each axis] → the surface is sized
  from the margin in one place, and the panel's own size and position are unchanged.
- [A brighter rim on a bright wallpaper is more visible than the old faint one] →
  that is what `plush` decided; the profile keeps the top edge the only bright part
  and fades it out by 78 % of the height.
