# Proposal

## Why

A burst multiplies a scroll by up to four, and nothing on screen says so. The reader
flicks the wheel and the view travels further each notch, with no explanation on the
panel: the only sign that the panel understood "go faster" is the text rushing past.
The one piece of chrome that is already there for exactly that moment — the 3 px
indicator that appears while the view moves — is the place to say it, and a number is
the honest way to say it.

## What Changes

- **While a burst is accelerating, the indicator becomes a capsule carrying the
  multiplier.** The thumb grows into a pill in its own place — so the view's position
  is still where the strip would have been — and the pill reads `1.5x`, `2x`, `3.5x`,
  up to the cap. It is the same accent ink as the strip, with the number in the panel's
  normal ink, and its radius is half its own height, so it is the strip deformed
  rather than a second widget.
- **It goes back to a strip when the burst ends.** The multiplier is read as the burst
  stands: once the burst's own window (400 ms) has passed without another step the
  panel is handed 1x, and the indicator is the 3 px strip again, fading on its usual
  clock.
- **The number is the multiplier, not a count of notches.** It is what the reader
  feels — this notch travels 2.5 times what one notch does — and the count would need
  a second word to be read at all.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the scroll indicator is a capsule with the burst's multiplier on it
  while a burst accelerates, and the plain 3 px strip otherwise.

## Impact

- `src/panel.rs`: the capsule's geometry, its label, and the branch in `draw`.
- `src/main.rs`: the burst's multiplier as it stands now, passed to the painter.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`.
- No new dependency, no new file at runtime, nothing on disk.
