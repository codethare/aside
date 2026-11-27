# Tasks: the cat as line art

## 1. The painter

- [x] **Step 1: RED — the line-art facts.** In `src/cat.rs` tests, assert the
  waiting cat is sparse: a body-interior sample `(0.30w, 0.70h)` and the head's
  centre are transparent, while the body's ground outline and the paw at
  `(w - 1, h/2)` are inked. Replace the silhouette's `lit > 2000` assertion with a
  line-art range and keep the pose-difference tests.
- [x] **Step 2: Run.** `cargo test cat::` -> FAIL.
- [x] **Step 3: GREEN.** In `src/cat.rs`: add `curve` (Catmull-Rom, no
  allocation) and `ring`; replace `BODY`/`RIM`/`EYE`/`TONGUE`/`MOUTH` with `INK`,
  `SHADOW`, `TONGUE`, `MOUTH`; move `mouth()` onto the head ring at 18 deg;
  replace `cat`, `silhouette`, `details` and `peek_pose` with the line contours;
  keep `paw`, `tail_start`, `mouth_at` and every clock.
- [x] **Step 4: Run.** `cargo test cat::` -> PASS. `cargo test panel::` -> the
  known metrics failure only.
- [x] **Step 5: Look.** `cargo run -- --preview /tmp/aside.ppm`, convert the
  `.cat.*.ppm` files to PNG, inspect each pose; three passes on the face, the paw
  ring and the lick's tongue until the cat reads at 260 px.
- [x] **Step 6: Commit.** `jj describe -m "feat: redraw the cat as line art"`.

## 2. The words

- [x] **Step 7.** `README.md`: the line-art cat, not a black cat.
- [x] **Step 8.** `docs/aside.1`: same, in the `mode` paragraph.
- [x] **Step 9.** `CLAUDE.md` rule 4: the cat is one-weight line art in the
  panel's ink.
- [x] **Step 10: Commit.** `jj describe -m "docs: describe the cat as line art"`.

## 3. Gate

- [x] **Step 11.** `cargo fmt --check` PASS; `cargo clippy --all-targets -- -D
  warnings` PASS; `cargo test cat::` 15 PASS; `cargo test` 308 PASS, 2 known
  non-failures. `groff` is not installed on this machine.

## Ledger

- Ruling: the design named an elliptical-ring painter for the mouth; the finished
  mouth is a small filled ellipse at the mouth point, which is a face detail, not
  a silhouette. No elliptical ring was needed.
- Ruling: the peek draws the head, ears and eyes only. A paw ring at the head's
  bottom read as a muzzle, not a paw.
- Ruling: the animal's `Act::stretch` went with the silhouette; line art has no
  body fill to stretch.
- The whole-branch review's focus: the light-desktop case is the dark offset's
  job and has no compositor or white-wallpaper preview on this machine. The
  surface path is unchanged and unverified without a compositor.
