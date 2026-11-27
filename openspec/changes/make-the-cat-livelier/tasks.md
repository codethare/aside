# Tasks: a modern, spring-eased cat

## 1. The model

- [x] **Step 1: RED.** In `src/cat.rs` tests: `blink()` is 0 at the start, 1 in
  its window and 0 after it; the reach overshoots by at most 3 % and settles on 1;
  the smile ends on the mouth under the rest tilt; the paw contact pixel is still
  inked.
- [x] **Step 2: GREEN — Act.** Added `tilt`, `tail_sway`, `ear`, `tongue_sway`
  and `blink`; `REST` carries the 0.10 rad tilt. Added `blink(since)`,
  `ease_out_back` and `spin`.
- [x] **Step 3: GREEN — pivots.** `on_head` turns the ears about the head's
  centre; `face` turns the eyes, nose, smile and whiskers about the mouth. The
  mouth point does not move.
- [x] **Step 4: GREEN — stance.** Head to `(0.65w, 0.34h)`; taller asymmetric
  body; a looped tail; ear twitch; a closed-eye stroke for the blink.
- [x] **Step 5: GREEN — motion.** Per-pose Act curves: arm/extend spring,
  irregular lick, two-part speak, sway and twitch. `hop`'s horizontal travel is a
  smoothstep.
- [x] **Step 6: GREEN — blink wake.** `stage_wake` returns the blink interval for
  `Waiting`, and only while the stage is visible and configured.
- [x] **Step 7: Run.** `cargo test cat::` 19 PASS; `cargo test panel::` -> the
  known metrics failure only.
- [x] **Step 8: Look.** Previewed every pose at 3x, plus a new `blink` preview
  frame; the looped tail, the rest tilt and the open peek face all read.
- [ ] **Step 9: Commit when asked.** `jj describe -m "feat: make the cat livelier"`.

## 2. The words

- [x] **Step 10.** README and man page: the tilted, spring-eased cat.
- [x] **Step 11.** `CLAUDE.md` rule 4: the two pivots and the blink.
- [ ] **Step 12: Commit when asked.** `jj describe -m "docs: describe the
  livelier cat"`.

## 3. Gate

- [x] **Step 13.** `cargo test` 312 PASS and the two known non-failures;
  `cargo fmt --check` PASS; `cargo clippy --all-targets -- -D warnings` PASS;
  `openspec validate make-the-cat-livelier --strict` valid. `groff` is absent on
  this machine.

## Ledger

- Ruling: the face turns about the mouth, not the head's centre, so `mouth_at`,
  the bubble's tail and its growth origin stay fixed under any tilt. The ears turn
  about the centre so their bases stay on the head ring.
- Ruling: the first blink waits 1.2 s, so the cat settles after landing instead of
  shutting its eyes on arrival. The preview's `waiting` frame is therefore
  open-eyed, and a `blink` frame at 1.3 s shows the closed one.
- Ruling: the waiting stage asks for a frame every 260 ms, which the idle tick
  caps at 250 ms. That is four frames a second for the whole panel while a cat is
  on screen, not a 16 ms animation loop. The cost is the blink's; the alternative
  was no blink.
- Fix found in the preview: the old body was a symmetric loaf that read as one
  mass; the chest now rises to the head and the rump is lower, and the tail's tip
  loops clear of the body.
- Review focus: the motion is a pure function of `since`, so the preview frames
  and live frames agree, but the stage's 250 ms sampling of a 260 ms blink can
  make the blink longer than 260 ms on a late compositor. The surface path is
  unchanged and unverified without a compositor.
