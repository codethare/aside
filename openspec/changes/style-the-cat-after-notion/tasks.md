# Tasks: the cat in Notion's language

## 1. The painter

- [x] **Step 1: RED.** In `src/cat.rs` tests: the paw contact pixel at
  `(w - 1, h/2)` is still inked, the body interior and head centre are still
  transparent, and a lick frame carries the orange accent.
- [x] **Step 2: GREEN — palette and weight.** `INK` -> `#e9e9e7`, `TONGUE` ->
  `#ffa344`, `SHADOW` -> `#191919` at 30 %, weight `0.012 * w`, underlay offset
  `0.004 * w`.
- [x] **Step 3: GREEN — waver.** In `Ink::curve`, offset each interior sample
  perpendicular to its segment by `0.0035 * w * sin(pi*t) * sin(2.399*i + 2*pi*t)`.
  Endpoints stay exact.
- [x] **Step 4: GREEN — paw and face.** Add `ellipse_ring`; draw the paw as an
  ellipse ring with two toe strokes, tangent at `(w, h/2)`; replace the mouth's
  "w" with a nose triangle and one smile curve.
- [x] **Step 5: Run.** `cargo test cat::` -> 16 PASS.
- [x] **Step 6: Look.** `cargo run -- --preview /tmp/aside.ppm`, inspect every
  pose; corrected the ellipse ring's coverage, tightened the mouth point to 25
  deg, and made the open mouth a flat orange oval.
- [ ] **Step 7: Commit when asked.** `jj describe -m "feat: style the cat after
  Notion"`.

## 2. The words

- [x] **Step 8.** `README.md` and `docs/aside.1`: name the Notion-style marker
  line and the orange accent.
- [x] **Step 9.** `CLAUDE.md` rule 4: the cat's palette, weight and flat waver.
- [ ] **Step 10: Commit when asked.** `jj describe -m "docs: describe the
  Notion-style cat"`.

## 3. Gate

- [x] **Step 11.** `cargo test` 309 PASS and the two known non-failures;
  `cargo fmt --check` PASS; `cargo clippy --all-targets -- -D warnings` PASS;
  `openspec validate style-the-cat-after-notion --strict` valid.

## Ledger

- Ruling: Notion's dark-mode translation (off-white ink, orange accent) rather
  than a paper-white card behind the cat. The stage is transparent chrome beside
  the panel; a white card would change the mode, not only the drawing.
- Ruling: the open mouth is the accent itself — a flat orange oval with the line
  round it — rather than a dark mouth with a small tongue. At 260 px the small
  tongue was a dot, and Notion uses flat colour as a shape.
- Ruling: the mouth point stays on the outline at 25 deg, not 18 deg, so the
  smile can run into it while the bubble's tail stays above the paw mitten.
- Fix found in the preview: `ellipse_ring`'s first coverage multiplied two
  half-plane clamps, leaving the mitten's ring at 4 % alpha. It now measures the
  edge distance in px, the same band the other strokes use.
- Review focus: the light-desktop case is the 30 % underlay's job and has no
  white-wallpaper preview on this machine. The surface path is unchanged and
  unverified without a compositor.
