# Design: the cat in Notion's illustration language

## Intent

Keep the cat's geometry — the stage, the landing, the paw contact, the mouth on
the outline, every pose clock — and change only how the line is drawn, so the cat
reads as a Notion-style illustration rather than as a diagram in the panel's ink.

## Goals

- A warm, flat, single-weight marker line with a hand-drawn waver.
- Notion's dark-mode palette: off-white ink and one orange accent, no gradient,
  no cast shadow.
- A simpler, softer face and a paw that reads as a paw, not a ring.
- The drawing still scales with the stage and still reads at 180 px.

## Non-goals

- A paper-white card behind the cat, a new surface, a new mode or key.
- Changing the landing, the paw contact, the bubble's tail or its growth origin.
- A second accent colour or any shading.

## The reference, translated

Notion's product illustrations are hand-drawn black-ink doodles on white, with
flat colour used once, rounded caps and simple geometric faces. In the panel's
dark chrome the faithful translation is Notion's dark-mode drawing: warm off-white
ink (`#e9e9e7`), a flat orange accent (`#ffa344`), no shading. A paper-white card
behind the cat was rejected: the stage is transparent chrome beside the panel, and
a white card would change what the mode is, not only how the cat is drawn.

| Token | Value | Use |
| --- | --- | --- |
| ink | `#e9e9e7` | every contour, eyes, nose, whiskers |
| accent | `#ffa344` | the tongue and the open mouth |
| underlay | `#191919` at 30 % | legibility on a light desktop |

## The line

- Weight `0.012 * S`, never under 1.5 buffer px. At 260 px that is 3.1 px: the
  marker weight of a Notion doodle, not a hairline.
- Rounded caps everywhere; no mitred joins.
- A deterministic waver of `0.0035 * S` perpendicular to each curve segment,
  windowed by `sin(pi * t)` so it is exactly zero at every control point. Joins
  with the head, the tail root and the paw stay exact; the line between them is
  hand-drawn rather than CAD-smooth. The waver uses no random source, so a frame is
  reproducible.
- The dark underlay is offset by `0.004 * S` and is not a shadow: it is one thin
  dark line under the ink, present only because the stage floats over an arbitrary
  desktop.

## The form

| Part | Change |
| --- | --- |
| Paw | an ellipse ring `0.052w x 0.040w`, its right edge at the stage's right edge; the leg meets its near edge; two short toe strokes cross the top |
| Face | dot eyes; a small filled triangle nose; one smile curve under it; two whiskers |
| Mouth | the point on the head's outline at 25 deg is unchanged; the open mouth is a flat orange oval with the line round it |
| Body, head, ears, tail | the same contours as the line-art change, with the new ink, weight and waver |

`paw(w, h)`, `tail_start`, `mouth_at`, the landing and the stage floor do not
change.

## Risks

- **The waver at small stages.** At 180 px the amplitude is 0.63 px; it may vanish
  into antialiasing. That is acceptable: a straight line at 180 px is better than
  a wobble that steers the silhouette.
- **The underlay reads as a shadow.** It is a single 1 px line at 30 %, not a
  blurred cast; if it reads as a drop shadow the answer is to thin it, not to
  remove it, or the cat disappears on a white desktop.
- **The paw's ellipse at the stage edge** is clipped by the buffer on its right
  half. That is the paw pressing on the pad; the tangent pixel remains inked.
- **The surface path is unchanged and still unverified without a compositor.**
