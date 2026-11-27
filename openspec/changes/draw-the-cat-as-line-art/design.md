# Design: the cat as line art

## Intent

Replace the cat's filled silhouette with one-weight line art in the panel's own
ink, keeping every geometric identity the cat mode already promises: the stage
lands with its right edge on the input card, the paw meets that edge at the card's
vertical centre, and the bubble's tail grows out of the cat's mouth.

## Goals

- A cat that reads as a drawing at 260 px and at 180 px: open contours, no filled
  body, no filled ears.
- One line weight, a fraction of the stage's side, so the drawing scales with the
  panel's size and buffer scale.
- The line stays legible over a light or a dark desktop.
- The mouth is a point on the head's outline, so the card-coloured bubble tail
  ends on the line rather than crossing a transparent head.
- The paw's ring is tangent to the stage's right edge at `(S, S/2)`.
- Every pose clock, the edge roll and the hop stay exactly as they are.

## Non-goals

- A new mode, key, surface, dependency, asset, or file on disk.
- Moving the stage, the landing, the tail's start, or the bubble's growth origin.
- A silhouette or a second ink style: this is one drawing, not a skin system.

## Constraints found in the code

- `cat::draw` receives a transparent shm buffer at buffer scale `s`, sized
  `S * s`. Shapes are placed in buffer px from fractions of `w`; the line width
  must be a fraction of `w` too, or it thins at scale 2.
- The buffer is cleared every frame. Nothing may be filled to occlude an earlier
  stroke: a filled head would be a black disc, not line art.
- `bubble_tail` is painted before the cat in the reply state. With a filled head
  it ends inside the face unseen; with a transparent head it must end on the
  outline.
- `mouth_at` feeds `mouth_in_surface` in `main.rs`, which is the bubble's growth
  origin. It may move along the head's boundary, but it must stay a point.
- `paw(w, h)` is the geometric contact returned to callers; it stays `(w, h/2)`.

## Approaches

1. **Keep the filled silhouette and add an outline.** Cheapest diff: draw the
   existing discs and capsules in a light ink and stroke their edges. But the
   filled body is the thing being removed; a light-filled body is a different
   sticker, not line art.
2. **Line art from the existing primitives.** Zero new abstractions: discs,
   ellipses, capsules and triangles as unfilled outlines. The head and paw are
   easy (rings), but the ears, body and tail become polygon chains whose joins
   depend on capsule overlaps; the body either reads as stacked ovals or needs
   many hand-placed capsules.
3. **Line art with a stroked-path sampler.** Add two small painters — a
   Catmull-Rom polyline and a circular ring — and draw the cat as named contours
   (body, head, ears, tail, leg, paw, face). The line has one weight, joins are
   round, and the shapes are described by the points that matter. Costs ~50 lines
   of painter code, no allocation.

**Picked: 3.** It is the smallest code that draws what was asked: contours, not
filled primitives. The sampler allocates nothing (it strokes each curve segment as
a capsule), so the per-frame cost stays proportional to the line's length, not the
window.

## The drawing

Landing identity, unchanged:

```
stage.x + S == panel.x + SHADOW
stage.y + S/2 == bar_centre(panel)
paw(S, S) == (S, S/2)
```

In stage-local buffer coordinates, with `w = h = S * s`:

| Part | Geometry | Line |
| --- | --- | --- |
| Head | ring centred `(0.62w, 0.38h)`, radius `0.16w` | 1 weight |
| Ears | two open `V`s, bases on the head ring at 205/245 deg and 295/335 deg, peaks at 225/315 deg at `1.45r` | 1 weight |
| Eyes | filled dots at `(hx +/- 0.38r, hy - 0.15r)`, radius `0.09r` | dot |
| Mouth | `(hx + cos 18deg * r, hy + sin 18deg * r)`: on the head's outline | spot |
| Body | one open Catmull-Rom contour from the head at 155 deg, round the back and rump, along the ground, up the chest, back to the head at 55 deg | 1 weight |
| Tail | Catmull-Rom from the rump out and up, curled at rest and straight up mid-hop | 1 weight |
| Front leg | from the chest to the paw, arced by `Act::reach`; paw ring centred `(w - 0.05w, h/2)`, radius `0.05w` | 1 weight |
| Whiskers | two short lines from the muzzle, a half weight | 0.5 weight |

The line's weight is `0.0095 * w`, never under 1 buffer px. The ink is
`#ebebf5` at full alpha; under it, offset `(0.005w, 0.005w)`, the same strokes are
painted in `#0e0e13` at 40 % so a light desktop does not swallow the line.

The mouth on the outline at 18 deg is what makes the transparent head work: the
bubble's tail, painted first in the card's own fill, ends exactly on the line the
cat then draws, so it reads as coming out of the mouth without a filled head to
hide its point. `mouth_at` returns that same boundary point.

Poses keep the existing `Act` values: the same reach, dip, mouth and tongue
numbers drive the same named parts. `Peek` draws only the head ring, the ears and
the eyes over the edge.

## Risks

- **Line legibility on a light desktop.** A light line alone disappears; the dark
  offset is the answer. It is painted first, so the visible line is still one
  weight of light ink.
- **Contour joins.** Catmull-Rom sampled into round-capped capsules can thicken at
  a sharp control point; the body uses gentle control points and the ear tips are
  the only sharp corners.
- **The preview tone.** `--preview` composites the stage over `BG_PROMPT`
  (dark). It verifies the light line; the light-desktop case is the dark offset's
  job and is checked by looking at the same frame over a light fill.
- **The surface path is unverified without a compositor.** This change touches the
  painter only; the stage lifecycle is unchanged.
