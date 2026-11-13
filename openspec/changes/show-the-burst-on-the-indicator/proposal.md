# Proposal

## Why

A burst multiplies a scroll by up to four, and nothing on screen says so. The reader
flicks the wheel and the view travels further each notch, with no explanation on the
panel: the only sign that the panel understood "go faster" is the text rushing past.
The one piece of chrome that is already there for exactly that moment — the 3 px
indicator that appears while the view moves — is the place to say it, and a number is
the honest way to say it.

A pill grown around the number was the first answer and it is the wrong one: the
indicator's job is to show *where* the view is, and a capsule as wide as a word takes
that shape away from the row it covers. The number belongs on the strip, at a size
that cannot be mistaken for a line of the answer.

## What Changes

- **While a burst accelerates, the multiplier is written on the strip.** The strip
  keeps its 3 px width and its length still shows the view's position; the number is
  centred on the strip's own column and on the thumb's middle, so it reads as the
  indicator's label rather than as a second widget. It reads `1.5x`, `2x`, `3.5x`, up
  to `4x`.
- **It is set smaller than the panel's text.** A face at 60 % of the panel's size, in
  the panel's text ink at the indicator's own strength: a short mark on the right
  edge, never a row of the answer, and it is clamped inside the card so it can never
  sit past the glass or outside the card's rows.
- **It goes when the burst does.** The number is drawn exactly while the multiplier
  stands above one — one frame after the burst's own window (400 ms) has passed
  without a step it is gone — and the strip carries on fading on its own clock.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the scroll indicator carries the burst's multiplier as a small
  number on the strip, and the strip keeps its shape and its length.

## Impact

- `src/panel.rs`: where the number goes, its size and its ink.
- `src/main.rs`: the burst's multiplier as it stands now, and a face at label size.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`.
- One extra face read at startup, on its own thread beside the panel's own font; no
  new dependency, no new file at runtime, nothing on disk.
