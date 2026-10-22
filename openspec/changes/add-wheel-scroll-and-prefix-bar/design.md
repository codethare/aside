# Design

## Context

Pointer frames already reach `State::pointer_frame` for the drag. `Viewport` holds
`top`/`height` in buffer pixels and a `following` flag. The bar is painted by
`draw_bar` from `input.text()`, with `preedit` appended and an optional right-hand
label; the card bodies all take one tone from `render::body`.

## Goals / Non-Goals

- Scroll from the pointer without a second scroll path, and make the prefix mode
  visible with the two things the bar already has (its tone and its line).

**Non-Goals:**

- Horizontal scrolling, kinetic/inertial scrolling, scroll on the replies' own
  scrollbar, or scrolling from the picker's ball state.
- A general prompt/hint language: one hard-coded instruction for one mode.

## Decisions

**1. One `Viewport::scroll_by(delta_px, total)` for the wheel.** It clamps to the
transcript and owns the `following` rule (up stops following, the bottom resumes),
which is the same rule `ensure_visible` already applies. The pointer handler only
converts an axis event into pixels, so a wheel, a touchpad and a future scrollbar
share one path.
*Alternative rejected:* calling `ensure_visible` with a computed row — it would
have to invent a highlight to scroll, and the scroll does not move the highlight.

**2. A notch is three lines; a continuous delta is pixels.** `discrete` is often
zero on touchpads (the crate says so), so the pixel value is the fallback and the
notch count is preferred when it exists: three lines is one screenful at eleven
notches, which is the usual rate.

**3. The hint replaces the input line and the tone changes; nothing is added to the
layout.** `draw_bar` takes a `hint` and a tone: when the hint is present it draws
the hint in place of the typed text, with no caret and no pre-edit. `render::body`
takes the colour again, so the one card that differs is the one painted
differently.
*Alternative rejected:* an extra row or an overlay above the bar — the bar is one
row tall by design, and the mode must not move anything.
