# Redraw the cat as line art

## Why

The cat is the reason to choose `mode = "cat"`, and it is currently a filled
near-black silhouette with a light rim. At the stage's 260 px it reads as a solid
sticker: the body, head and limbs are discs and capsules, and the face is three
ink blobs. The panel's chrome is a quiet, thin-rimmed drawing; the cat should be
drawn the same way rather than pasted on top of it.

## What changes

- The cat's painter in `src/cat.rs` draws one continuous-weight line instead of a
  filled silhouette: an open body contour, a head ring, open ear lines, a curved
  tail, a front leg reaching the pad, and a small paw ring tangent to the stage's
  right edge.
- The face is line art too: two ink dots for eyes, a nose, a mouth, and two
  whiskers. The mouth moves onto the head's outline so the bubble's tail meets it
  without crossing the transparent head.
- The line is the panel's light ink with a faint dark drop offset under it, so it
  reads on a light desktop as well as a dark one.
- The geometry does not move: the stage's landing, the paw contact at
  `(S, S/2)`, the bubble's tail start, the mouth used as the bubble's growth
  origin, the edge roll, the hop and every pose clock are unchanged.
- README, the man page and the chrome rule say line-art cat instead of black cat.

## Impact

- Affected specs: `cat-stage` (the drawing), `chat-panel` (the bubble's tail now
  ends on the head's outline).
- Affected code: `src/cat.rs` (painter only), `src/main.rs` (preview comment and
  compositing tone), `README.md`, `docs/aside.1`, `CLAUDE.md`.
- No new dependency, no asset, nothing on disk, no new surface, no new key.
