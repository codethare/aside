# Design

## Context

`aside`'s frame is the old `wlmenu` one: a single `Panel`, a flat `t²` lit
falloff, `SHADOW` 24, `BG_NORMAL` behind the replies and `BG_CARD` behind each of
them. `wlmenu` has since split into two cards with a signed-distance shadow, moved
to plush's shadow and hairline numbers, and draws its rail only for a prompt
label. The reply blocks carry a card fill and 8 px of padding from the previous
change; the highlight is a 2 px `BG_SEL` rail.

## Goals / Non-Goals

- Chrome, tones and anatomy identical to the current `wlmenu`; one body tone
  across both cards; no mark in the margin.
- The reveal is a one-shot, 160 ms, and cannot perturb layout.

**Non-Goals:**

- Animating the panel's own size (a configure round-trip per frame), the ball, or
  the panel opening and closing.
- A fade, a slide, or any motion other than the card's height.
- Porting `wlmenu`'s `-p` label capsule, its selection colour behaviour for lists,
  or its row grid: `aside` has blocks, not rows.

## Decisions

**1. Port `wlmenu`'s shadow and body verbatim**, including the `Shape` /
`outline_gap` signed-distance helper and the two profile tables, with only the
`SHADOW` constant and the panel list differing. Both products then share the
chrome numerically, which is what "align with `wlmenu`" has to mean.
*Alternative rejected:* keeping the per-row span shadow and adding a second panel —
the corner falloff is exactly what the new `wlmenu` fixed, and two independent
passes would clear each other's bands.

**2. One body tone by deleting the per-block fill, not by recolouring it.** The
bar and the replies card both take `BG_PROMPT`, a reply block paints no fill, and
`Blocks` loses its padding. The card outline is the only container, which is also
what makes the reveal possible: there is one rectangle to grow.
*Alternative rejected:* keeping `BG_CARD` at the bar's tone — two identical tones
with two outlines is either invisible or a seam, and it doubles the shadow work.

**3. The reveal clips the card's height, anchored at the bar.** The panel surface
takes its final size when the card appears, so nothing resizes per frame; the card
is painted with height `ease(t) × full`, its rows positioned from that card's
top, and everything below the current height is clipped. Text therefore travels
upward out of the bar rather than being wiped in place.
*Alternative rejected:* animating the surface height — a `set_size` plus configure
round-trip every 16 ms, which is what makes layer-surface animations stutter.
*Alternative rejected:* a clip anchored at the card's top (a wipe): it reads as a
mask, not as the field producing the card.

**4. The animation is a deadline, not a frame counter.** `reveal_start: Instant`
and `reveal_at(elapsed)` in `main`, like `blink_at`, so a late frame lands at the
right height. While it runs, the event loop wakes every 16 ms; when it ends the
wake returns to its normal period and the frame is painted once more at full
height.

**5. The bar's position is a share of the output, not a constant.** The margin is
`output_h * 38 / 100`, clamped to at least the shadow margin. A constant could not
sit "just below the centre" on every screen; the height cap is derived from the
same number, so a tall answer stops before the top edge instead of the anchor
fighting the cap.

## Risks / Trade-offs

- [Two cards cost roughly two bodies and one shadow pass] → the shadow is one pass
  over both cards and the bodies are whole-row span writes; the frame is still per
  visible row plus two outlines.
- [A card whose height changes every 16 ms re-runs its hairline profile] → the
  profile is two table reads per row; the card is at most 64 rows.
- [The highlight as ink alone is subtle on a translucent panel] → `FG_SEL` is
  opaque white on a tone no other line uses, and the 2 px inset moves the first
  glyph, which is visible at a glance; the live checklist asks for exactly that
  check.
- [A percentage margin puts the panel over whatever the compositor draws at
  mid-screen] → that is the request: the field belongs where the eye is.
- [Selection with no mark in the margin removes the last cue that a line is
  selected] → the spec's copy scenario and the live checklist are what keeps it
  honest; if it proves too subtle, the ink inset grows, not a rail.
