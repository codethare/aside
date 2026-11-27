# Restyle the cat after Notion

## Why

The cat is now one-weight line art, but its language is still the panel's: a thin
cool-white line, a perfect ring for a paw, a geometric "w" for a mouth. Notion's
illustrations are warmer and flatter: an off-white marker line of one rounded
weight, a small hand-drawn waver, simple dot faces, flat colour used once, and no
shading. The cat should borrow that language while keeping the geometry the panel
depends on.

## What changes

- The ink becomes Notion's dark-mode off-white (`#e9e9e7`) with Notion orange
  (`#ffa344`) as the one accent, on the tongue.
- The line is one weight at 1.2 % of the stage's side, with a deterministic
  hand-drawn waver that is zero at every join and at the paw contact.
- The stroke is flat: no gradient, no cast shadow. A thin dark underlay (30 %)
  stays only so a light desktop does not swallow the drawing.
- The paw becomes a rounded mitten: an ellipse ring with two toe strokes, its
  right edge still tangent to `(S, S/2)`.
- The face is simpler and warmer: dot eyes, a small filled nose, one curved smile
  under it, and two whiskers. The open mouth is a flat orange oval with the line
  round it.
- The mouth stays on the head's outline, so the bubble's tail still meets it.

## Impact

- Spec: a new `cat-stage` delta on the drawing's palette and form.
- Code: `src/cat.rs` only. No new key, surface, dependency, asset or file.
- Docs: `README.md`, `docs/aside.1`, `CLAUDE.md` rule 4 name the style.
