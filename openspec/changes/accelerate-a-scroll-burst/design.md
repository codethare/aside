# Design

## Context

`State::wheel` turns an `AxisScroll` into fractional buffer pixels (`axis_delta`),
carries the fraction (`whole`) and hands the result to `Viewport::scroll_by`.
`State::wheel_size` is the `Ctrl` chord's path and is separate. The axis source —
wheel or finger — is already remembered in `axis_source`, so the handler knows which
kind of device it is answering.

## Goals / Non-Goals

- Make a repeated, quick scroll travel farther, and make that distance come back to
  one step the moment the reader slows down.
- Non-Goals: kinetic scrolling after the fingers lift, easing a single notch,
  accelerating the `Ctrl` chord, a configuration key, or any per-device tuning the
  reader has to discover.

## Decisions

**1. A burst is a run of steps closer than 150 ms; a pause of 400 ms ends it.** 150 ms
is faster than any deliberate notch-by-notch scroll (a wheel detent is 200–330 ms
apart in careful use) and slower than a spin (10–20 notches/s is 50–100 ms apart), so
the line falls where intent changes: careful use never accelerates, and a spin does
from its second notch. The larger 400 ms window is only for the touchpad, where a
"quick flick again" is a new gesture and a gap of 150 ms is what separates one gesture
from the next.

**2. Growth is linear in the step count, capped at four.** `1 + 0.5 (n - 1)` gives
1×, 1.5×, 2×, 2.5× … so the second notch of a spin is the smallest jump a reader can
notice, and the seventh is the cap. Four times five rows is 20 rows — about 600 px,
more than the card is tall — which is as far as a notch can go without the eye losing
the line it was on. A multiplicative curve (the Firefox shape, ×10 at once) is what
accelerated scrolling feels bad with: the jump is unreadable and cannot be steered.
*Alternative rejected:* a rate proportional to the wheel's speed in px/s — the
compositor's axis values are not a speed (they are not monotone in time across
compositors), and the step count is what the reader is actually doing.

**3. A touchpad gesture is one step.** Within a gesture the step count does not move
(`since < 150 ms` keeps the current step), so one slide keeps the 1:1 mapping that
makes a touchpad a touchpad; each new gesture within 400 ms of the last is one more
step, so repeated quick flicks accelerate exactly like repeated wheel notches.

**4. The burst's clock is the panel's, not the compositor's.** `wl_pointer.axis`'s
timestamp is zero on the events that carry `value120` and `discrete` (the toolkit
leaves it unset), so the burst is timed off `std::time::Instant` taken when the event
is handled. It is a perception window, not a measurement of the wheel, so the
few hundred microseconds of dispatch in it do not matter.

**5. A pause resets rather than decays.** macOS lets the multiplier fall away
smoothly; a reset is what keeps the promise that the view can always be aimed again —
stop for 150 ms and the next notch is five rows, with nothing left over. The cost is
that a burst that breaks for a quarter second restarts at 1×, which is the honest
reading of "I slowed down".
