# Design

## Context

`State::phase` moves Idle → Thinking → Answering and `apply_size` sets the surface
from it. The painters are three static functions: the bar (inside `draw`), the ball
(`draw_ball`), and the bar plus the replies card (`draw`, with the `reveal` that
already exists). A layer surface cannot be resized per frame without a configure
round-trip, so an animation of the surface's own size is not available.

## Goals / Non-Goals

- The three transitions as one card changing shape, inside one buffer, at the larger
  state's size; the settled path untouched.

**Non-Goals:**

- Blur, scale or opacity of the surface as a whole (needs the compositor), bouncy
  overshoot beyond a couple of percent, or animating anything else (typing, scroll).
- A closing that leaves the process alive for anything but the animation's length:
  `Esc` starts the closing and the loop exits when it has run.

## Decisions

**1. One interpolated card, two cross-faded contents.** `panel::Look` carries the
state being drawn, the state it came from and the progress; a transition paints the
lerp of the two states' cards and fades the content being left out over the first
half and the content arriving in over the second, so the card is never seen empty
and the arriving text is only drawn once the card is wide enough to hold it. Both
contents are the existing painters with a `fade` argument, which is `render::dim` on
the colour they already receive.
*Alternative rejected:* a per-pixel cross-fade of two whole frames — it doubles the
paint cost and needs the fade inside every blitter.

**2. The surface takes the larger of the two states for the duration.** Bar→ball
runs on a 640×60 surface (the ball's 60 px height is taller than the bar's band),
and ball→panel on the panel's own size, which is already the larger one. When the
transition ends the surface is set to the target once: a single shrink, hidden
behind the settled frame.
*Alternative rejected:* resizing the surface per frame — a configure round-trip per
frame, which is what makes layer-surface animation stutter.

**3. Opening and closing are two tracks of one card, not two animations.** `Look`
carries a width fraction (`spread`) beside the height one (`rise`): the card's width
shrinks to nothing about its own centre and its height to a 2 px line anchored at
the bottom. Opening runs the spread first and the rise second, closing the rise
first and the spread second, and the content fades by `rise × spread`, so nothing
is drawn on a line or on a point. `main` reads both from one clock
(`enter_tracks`/`close_tracks`), so there is no state machine to get out of step.
*Alternative rejected:* a slide from below — it needs clipping outside the buffer,
and `set_span` is not written for out-of-range rows.

**4. The spring is one closed form, next to the reveal.** `morph_at(ms)` is a
damped cosine normalised to reach exactly 1 at the deadline, so a late frame lands
at the right shape and the last frame is the settled one.
*Alternative rejected:* a simulation integrated per frame — stateful, frame-rate
dependent, and untestable without a clock fake.

## Risks / Trade-offs

- [Text fading needs the fade threaded into three painters] → one `dim` helper and
  one extra `f32`; the static callers pass 1.0, and the settled-frame test pins that
  a `t = 1` frame ignores where it came from.
- [The bar's text is drawn on a card that is shrinking under it] → it fades out over
  the first half of the morph, which is also when its card is still wide enough for
  it.
- [A transition that starts while another is running (Esc during the morph)] →
  starting a morph always replaces the current one and settles the previous target;
  the panel never waits for an animation to end.
