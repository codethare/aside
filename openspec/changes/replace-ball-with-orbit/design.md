# Design

## Context

`Phase::Thinking` owns `panel::State::Ball`: `size_for` makes it a 60 × 60 square,
`ball_card` puts a circle in it, `draw_ball_squares` blinks four cells, `blink_at`
picks which, and two morphs (bar→ball, ball→panel) exist to get in and out of it.
`render::body` paints the rim row by row from the profile tables; there is no notion
of a position *along* the outline.

## Goals / Non-Goals

- The bar, unchanged in size, with a light that reads as running around its edge.

**Non-Goals:**

- A glow wider than the rim (a 2 px band at most), blur, or a second colour: the
  light is the primary ink over the rim.
- Keeping the ball for anything (a settings toggle, a smaller window, …).
- Lighting the answering panel's rim: the replies arriving is the indicator there.

## Decisions

**1. The light walks the outline as a polyline.** `render::outline(panel)` returns
the card's rim pixels in order — top edge, right side, bottom edge, left side, with
the corner rows taken from `rounded_span` — and the orbit painter lights the points
whose fraction along that polyline is behind the head. One polyline of about 1,300
points per frame, built from the same outline the body and the shadow use, so the
light cannot drift off the edge.
*Alternative rejected:* a parameterised arc computed per pixel — the corner arcs
make the fraction-per-pixel arithmetic longer than walking the pixels themselves,
and it would disagree with `rounded_span` at the corners.

**2. `State::Ball` is deleted rather than left unused.** Thinking becomes
`State::Bar`, so `size_for(Thinking)` returns the bar's size, `set_phase` starts a
morph only when the *shape* changes, and the bar→ball morph and its duration go.
Two states, two sizes, one morph (bar↔panel).
*Alternative rejected:* a `Look` flag for "thinking" while keeping the Ball state —
it would leave a state nothing can reach.

**3. The light is drawn after the body, over the rim.** `body` paints the rim from
the profiles as it always has; the orbit is an additive pass of white at a fraction
of its alpha, so the light passes the lit top edge and the dark bottom one alike and
never needs the profiles to know about it.

## Risks / Trade-offs

- [A 1 px rim gives a thin light] → the band is two pixels wide (the rim and the one
  inside it) when the card is tall enough, which reads as a highlight rather than a
  line.
- [A lap every 1.4 s could read as slow at first glance] → it is deliberately calm;
  the number is one constant.
- [Deleting the ball removes the panel's only size change besides the answer] → the
  morph tests that covered the ball are replaced by the new shape-change tests; the
  ball's tests go with it.
