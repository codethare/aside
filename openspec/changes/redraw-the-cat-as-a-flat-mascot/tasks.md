# Tasks: a flat paper-and-ink cat

## 1. The fill

- [x] **Step 1: RED.** The tests now expect paper interiors at the body and head,
  ink on the ground line, head ring and paw contact, the coral accent on the lick,
  and ink at the mouth. The lit range moved from a sparse line to a filled mascot.
- [x] **Step 2: GREEN — primitives.** Added `Ink::polygon` (scanline, fixed stack
  intersections) and `sampled` (Catmull-Rom into a stack array, no allocation).
- [x] **Step 3: GREEN — materials.** `INK`, `PAPER`, `EYE`, `ACCENT`, `BLUSH`; three
  line weights; the underlay and the two-pass cat are gone.
- [x] **Step 4: GREEN — layers.** Tail, arm, body fill and outline, sitting legs,
  ground paw, ears, head, mitten paw, face; the tail loop and modern stance stay.
- [x] **Step 5: GREEN — face.** Big eye whites, ink rings, vertical pupils,
  highlights, gaze, blush, coral nose, ink smile, two whiskers, curved blink.
- [x] **Step 6: GREEN — peek.** The filled head, ears and face rise over the edge
  with the peeking gaze.
- [x] **Step 7: Run.** `cargo test cat::` 19 PASS; `cargo test panel::` -> the
  known metrics failure only.
- [x] **Step 8: Look.** Previewed every pose at 3x and a 180 px box-filtered
  render; the face, the mitten, the ground paw, the leg lines and the tail loop
  all read at the floor.
- [ ] **Step 9: Commit when asked.** `jj describe -m "feat: redraw the cat as a flat mascot"`.

## 2. The words

- [x] **Step 10.** README, man page and `CLAUDE.md` rule 4: the paper fill, the
  ink hierarchy, the coral accent and the gaze.
- [ ] **Step 11: Commit when asked.** `jj describe -m "docs: describe the flat
  mascot"`.

## 3. Gate

- [x] **Step 12.** `cargo test` 312 PASS and the two known non-failures;
  `cargo fmt --check` PASS; `cargo clippy --all-targets -- -D warnings` PASS;
  `openspec validate redraw-the-cat-as-a-flat-mascot --strict` valid. `groff` is
  absent on this machine.

## Ledger

- Ruling: the first pass made the mitten, the body and the head one flat cream
  mass with no legs. The preview showed a bean; added a ground paw, a grounded
  front-leg line and a folded hind-leg line. Both leg lines and the ground paw are
  tucked while `tail_up` (the hop), so a leaping cat is not sitting mid-air.
- Ruling: the open mouth was first a coral oval on the outline; at 260 px it read
  as a clown nose. It is now an ink mouth with a coral tongue, offset a little
  inside the outline.
- Ruling: the far-side whisker crossed the cheek like a wire. It is gone; two
  whiskers on the panel side remain, at the detail weight.
- The body is scanline-filled from a 10-samples-per-segment polyline in a fixed
  `[(f32, f32); 128]`; the ink outline is the same contour, so the fill and the
  line cannot drift apart by more than the hand-drawn waver.
- Review focus: the cream fill is the contrast on a dark desktop and the ink
  outline is the contrast on a light one; the light case has no compositor
  preview on this machine. The 180 px box-filtered render was the floor check.
  The surface path is unchanged and unverified without a compositor.
