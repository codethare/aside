# Proposal

## Why

The replies now scroll at a sane rate, but a long transcript still has to be crossed
a notch at a time: five rows is 30 notches for a page, and a reader who wants the top
of a long answer spins the wheel for seconds. Every pointer-driven surface a reader
already uses answers that with acceleration — macOS and Windows' precision touchpads
accelerate a wheel's distance with the speed of the spin, and Firefox exposes the same
idea as `mousewheel.acceleration` — so a repeated, quick scroll travelling farther is
the behaviour they expect, and its absence reads as the panel being stubborn.

## What Changes

- **A burst of quick scrolls accelerates.** Steps that arrive faster than 150 ms apart
  are counted as one burst, and the burst's nth step moves the view
  `1 + 0.5 (n - 1)` times one step's distance — 1×, 1.5×, 2× and so on, capped at 4×
  so a fast spin never outruns the eye. A pause of 150 ms or more ends the burst, so
  a single deliberate notch is always exactly five rows.
- **A touchpad gesture is one step.** A run of touchpad events separated by less than
  150 ms is one gesture, so one slide still moves the view by the distance the fingers
  moved; it is repeated quick slides — each one a gesture, each within 400 ms of the
  last — that accelerate, which is what a reader does to cross a long answer.
- **`Ctrl` with the wheel does not accelerate.** That chord resizes the panel, which
  is a different verb and must stay one size-step per notch.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: a scroll's distance grows with the speed of the burst it belongs to,
  within a cap, and a pause returns it to one step.

## Impact

- `src/main.rs`: the burst's clock and its step count, and the multiplier applied to
  `axis_delta`'s distance.
- `src/panel.rs`: nothing — the indicator is drawn from the view's offset, which
  needs no knowledge of how far one event moved it.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`.
- No new dependency, no new file at runtime, nothing on disk.
