# Design

## Context

`render::scroll_thumb` gives the thumb's `(y, h)`; `panel::draw` paints a 3 px
`LABEL_ACCENT` rectangle at the card's right edge, dimmed by the indicator's ink.
`render::fill` draws a rounded rectangle in a tone without the plush rim — the shape
a tint takes — and `render::draw_text` / `render::measure` are the panel's text path.
The burst lives in `State::burst`; `panel::draw` sees none of it, and takes the scene
it paints as flat parameters.

## Goals / Non-Goals

- Say the burst's multiplier on the chrome that already exists, and say nothing when
  there is no burst.
- Non-Goals: a tooltip on hover, a number for the touchpad's gesture count, a tween
  between the strip and the capsule (the burst's own window is the boundary), any new
  colour token, or a capsule while the view is merely scrolling slowly.

## Decisions

**1. The capsule is the thumb, not a second widget beside it.** Its `y` is the
thumb's, its height is `max(row_h, thumb_h)`, and it is right-aligned inside the card,
so the reader keeps reading the position from it exactly as from the strip. A pill
somewhere else on the panel would say "speed" while the strip said "where", which is
two things to look at during a scroll.
*Alternative rejected:* a fixed pill in a corner — it has no relation to the thumb and
would occlude a corner of text permanently while scrolling.

**2. Its radius is half its height, filled with the strip's own accent ink, and the
number is drawn in the panel's normal ink.** That is what makes it a capsule rather
than a small card: no rim, no shadow, no band — `render::fill`, which is the tint
helper the code blocks already use. The accent is dark slate and the card's tone is
dark grey, so the glyphs take the bright normal ink, which is the one pairing that
reads at 8 px of padding without a new colour token.
*Alternative rejected:* the glyphs in the card's own tone — invisible on the accent;
and a card-tone capsule with accent glyphs — the pill then reads as a hole in the glass
rather than as the strip grown into a label.

**3. The number is the multiplier, one decimal only when it is not whole.** `1.5x`,
`2x`, `3.5x`, `4x`. The burst's own steps are what a reader would have to count, and
the multiplier is what they can see: this notch travels two and a half times as far.
`x` rather than `×` because the source stays ASCII, and every face in the chain has it.

**4. The panel is handed the multiplier as it stands, not the last burst.** `State`
answers `1x` once 400 ms have passed without a step, so the capsule becomes a strip
again on the frame after the burst's own window — the same clock that ends the burst —
and the strip then fades on the indicator's hold and fade. One clock, two shapes, no
second timer.
*Alternative rejected:* holding the capsule for the indicator's own 900 ms — it would
say "4x" for half a second after the reader stopped, which is a lie about a live state.

**5. Nothing changes for the touchpad's single slide.** One gesture is one step, so the
multiplier is 1x, so the indicator stays a strip: the capsule only ever appears for a
burst, which is the only time there is a number to show.
