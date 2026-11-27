# Redraw the cat as a flat paper-and-ink mascot

## Why

The current cat is a thin open line on a transparent surface: a wireframe animal
with no body, no eye mass and no colour of its own. Beside a dark translucent
panel it reads as a diagram, not a mascot. Owl-Listener's `illustration-style`
asks for a deliberate geometric/organic, flat/dimensional and line-style choice;
MengTo's `illustration-outlined-cartoon` shows the choice that fits: a flat fill
inside a warm ink line, with a big face and one accent. `rossark-motion-skill`
adds the missing life: gaze, blink, head tilt and no teleporting.

## What changes

- The cat becomes **flat paper and warm ink**: a cream body and head fill, an
  ink outline in three weights (silhouette, part, detail), and one coral accent
  on the inner ears, nose, blush and tongue. The open mouth is an ink shape with
  that coral tongue. No gradients, no cast shadow, no underlay.
- The eyes become the cartoon face: large white eyes with vertical pupils, a
  white highlight, a gaze that follows the pose, and a curved ink stroke for a
  blink. The head is cocked; the blush and whiskers are flat marks.
- Fills are layered back to front: tail, arm, body, ears, head, paw, face. The
  body is a filled closed Catmull-Rom contour, not a stroked outline. The tail
  and arm are paper strokes with an ink border.
- The geometry does not move: stage, landing, paw contact at `(S, S/2)`, mouth on
  the outline, bubble tail and growth origin, edge roll, hop, pose deadlines and
  the existing spring/irregular timing.
- README, man page and `CLAUDE.md` rule 4 describe the flat mascot.

## Impact

- Code: `src/cat.rs` (the painter, a polygon fill, a sampled curve), tests, and
  no other module. `src/main.rs` only keeps its existing preview and stage wake.
- Spec: a `cat-stage` delta for the flat fill, the ink hierarchy and the face.
- No new key, surface, dependency, asset or file; nothing is written to disk.
- The light desktop is no longer a special case: the paper fill itself carries
  the contrast, so the dark underlay goes away.
