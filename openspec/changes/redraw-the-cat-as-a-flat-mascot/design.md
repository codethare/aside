# Design: a flat paper-and-ink cat

## Intent

Give the cat a material: flat cream paper inside a warm near-black line, a big
expressive face, and one coral accent. Keep every geometric contract and the
motion model already in place.

## The art direction (Owl-Listener: illustration-style)

| Axis | Choice |
| --- | --- |
| Geometric vs organic | organic: rounded contours, one looped tail, no straight limbs |
| Flat vs dimensional | flat 2D: one fill per shape, no gradient, no cast shadow |
| Detailed vs minimal | minimal-medium: a face, a chest, two toe marks; no fur, no hatching |
| Line style | warm near-black ink, rounded caps, three roles: silhouette 1.15 %, part 0.8 %, detail 0.55 % of the stage side |
| Colour | paper `#f3ecdf`, ink `#2b2520`, coral `#ef836d`, eye white `#fffdf8`; coral is the only accent, on the inner ears, nose, blush and tongue; the open mouth is ink with that tongue |

This follows MengTo's `illustration-outlined-cartoon` traits: every flat fill sits
inside an ink line, the silhouette is the thickest, parts are thinner, details
are thinnest, and the face carries big eyes and blush. It is not the "ink sketch"
style: no tapering brush, no hatching, no watercolour.

The **no-ai-design-slop** pass: the old thin line could be pasted into any panel;
the paper mascot is specific to this product — it holds the input, it has a body
with a chest and a paw, and its one accent is used by the mouth it answers from.
The motion stays explanatory: peek, hop, reach, lick, speak, return.

## Layer order (back to front)

1. **Tail**: a paper stroke inside an ink stroke (ink width = paper + 2 * outline),
   so both edges are inked in one pass. The root is covered by the body.
2. **Arm**: the same two-pass stroke, drawn before the body so the body hides the
   shoulder.
3. **Body**: a filled Catmull-Rom contour closed across the neck (the closing
   chord is under the head), then its open ink outline. The fill is sampled into
   a fixed stack array and scanline-filled; no allocation.
4. **Ears**: a paper triangle and two ink outer edges, drawn before the head so
   the head fill hides the base. A small coral inner triangle is drawn after the
   head ring.
5. **Head**: a paper disc and an ink ring. The fill hides the body's neck ends and
   the ear bases.
6. **Paw**: a paper ellipse, an ink ring, two ink toe marks. Its right edge is the
   stage's right edge.
7. **Face**: eye whites, ink rings, vertical pupils, highlights, blush, nose,
   smile, whiskers; all rotated about the mouth, so the bubble's point is fixed.

## Face

- Eye centres `(hx +/- 0.38r, hy - 0.12r)`, white radii `0.22r x 0.27r`, ink ring
  at the part weight.
- Pupils `0.08r x 0.16r`, offset horizontally by `gaze * r`; the highlight is a
  small eye-white dot. `gaze` is per pose: +0.10 while peeking, +0.08 reaching,
  about -0.12 at the bottom of the lick (down at the paw), +0.04 at rest.
- Blink: a curved ink stroke across the eye at the part weight, not a dot.
- Nose: a small coral triangle. Smile: an ink curve from below the nose to the
  mouth point. Blush: two coral ellipses at about 33 % alpha. Whiskers: two thin
  ink strokes on the panel side; the far side is left clean.
- The face rotates about the mouth; the head ring and the filled body do not, so
  `mouth_at` stays exact.

## Motion (rossark)

The spring reach, irregular lick, two-beat speak, blink and tail sway stay. The
new gaze makes the eyes act with the pose. No idle loop is added; the waiting cat
still uses the 250 ms idle tick for its blink.

## Risks

- **Fill aliasing.** The body polygon is scanline-filled; the 3 px ink outline
  covers the hard edge. The sampled fill and the wavered outline can differ by a
  pixel; the outline is wider than the waver.
- **The paw at the stage edge.** The mitten is clipped by the buffer; its ink ring
  is clipped too, which reads as the paw pressing the card. The tangent pixel
  stays inked.
- **The cream on a cream desktop.** The ink outline, not the fill, carries the
  silhouette on a light desktop; on a dark one the fill carries it.
- **The face at 180 px.** The pupils are about 2.6 px wide at the floor; the ink
  rings hold the eye shape. The preview at 260 and a 180 px render are the check.
- **The surface path is unchanged and still unverified without a compositor.**
