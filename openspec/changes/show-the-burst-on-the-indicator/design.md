# Design

## Context

`render::scroll_thumb` gives the thumb's `(y, h)`; `panel::draw` paints a 3 px
`LABEL_ACCENT` rectangle at the card's right edge, dimmed by the indicator's ink.
`render::draw_text` / `render::measure` are the panel's text path and take a
`PanelFont`; the panel builds its face once, at the configured size times the output's
scale, on a thread that overlaps the compositor handshake. The burst lives in
`State::burst`; `panel::draw` sees none of it.

## Goals / Non-Goals

- Say the burst's multiplier on the chrome that already exists, and say nothing when
  there is no burst.
- Non-Goals: a pill, a tooltip on hover, a tween, a number for the touchpad's gesture
  count, any new colour token, or a number while the view is merely scrolling slowly.

## Decisions

**1. The number is drawn in the strip's own column, centred on the thumb's middle.**
The strip is the thing being described, so the label sits on it: centred horizontally
on the 3 px column and vertically on the thumb's middle, so it moves with the position
it explains. The strip is painted first and the number over it.
*Alternative rejected:* the capsule that was built first — as wide as a word, it stops
reading as the view's position and hides a slice of the answer's last column; and a
fixed readout in a corner, which has no relation to what it describes.

**2. Set at 60 % of the panel's size, in the panel's text ink, at the indicator's own
strength.** The panel's own size would make a 4-character mark as tall as a line of
the answer, which is exactly what "must not disturb the text" forbids; 60 % makes its
glyphs shorter than half a row, and the ink is the same one the answer's text uses, so
it reads as chrome drawn on the card rather than as a second typeface.
*Alternative rejected:* the strip's own accent ink — it is dark slate, legible as a 3 px
bar and muddy as small text; and a brighter colour, which would be a token added for
one label.

**3. A second face, loaded at startup beside the panel's own.** The painter takes a
`PanelFont`, the font cache is per face, and there is no per-call size: a smaller
number means a smaller face. It is spawned on its own thread in the same breath as the
panel's font, so the handshake hides the read, and it fails soft — no label face, no
number, the strip as it was.
*Alternative rejected:* scaling the glyph planes at blit time — a resampler in the text
path for one label, with worse quality than a real 12 px rasterization.

**4. One clock, two marks.** `State` answers the multiplier as it stands — one once
400 ms have passed without a step — so the number is drawn while the burst is live and
gone the frame after it ends, while the strip keeps its own hold and fade. No second
timer, no tween, no lingering number over a scroll that has stopped.
