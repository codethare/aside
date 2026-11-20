# Tasks

## 1. A frozen leaving timeline

- [x] 1.1 `Leaving` (or a pure function) returns the frozen start scroll and the
  exit side from `(elapsed, since, travel, s)`: `age = elapsed - since` read once, not
  re-evaluated per frame — verify: a test places the leaving start just before a
  marquee turn and steps through the exit; the side and the start scroll never change
  across frames, where the current `leaving_offset` would flip
- [x] 1.2 `SUCK = DECOMPOSE + EXTRACT`; the clearing in `tick` uses it — verify: a
  unit test pins the total and the stage boundaries; a frame at `SUCK` has no ink in
  the pending bar

## 2. Particles

- [x] 2.1 Seeds: for each visible glyph, 8 particles with a deterministic offset from
  `(byte index, particle index)`; a hard cap of 480; no allocation per frame — verify:
  a test on the seed helper is stable across calls and another asserts the count is
  capped for a long line
- [x] 2.2 Decompose: glyph alpha falls from 1 to 0 over `DECOMPOSE` while particle
  alpha rises; positions stay at the frozen glyph boxes with a small seeded jitter —
  verify: a frame test at `0`, mid-`DECOMPOSE` and `DECOMPOSE` shows glyph ink falling,
  particle ink rising, and no movement of the line's box
- [x] 2.3 Extract: each particle accelerates toward the frozen edge with a seeded
  delay, fades out by `SUCK`, and is clipped to the bar's rows and limit — verify:
  frame tests at the start and end of `EXTRACT`, and a clip test with the panel's row
  range

## 3. Integration and feel

- [x] 3.1 `--preview` gains a leaving frame (or uses the existing scripted one) so the
  two stages can be compared without a compositor — verify: the frame shows particles
  at the glyph positions and a second frame shows them leaving one edge
- [x] 3.2 Typing during the leaving still drops the question at once; a short question
  decomposes and leaves left; a long one leaves the side it was travelling at the
  frozen instant — verify: the existing typing/marquee tests plus new short-question
  and frozen-side tests
- [x] 3.3 Run the gate — verify: `cargo test` shows only the pre-existing
  font-environment failures, `cargo fmt --check` and `cargo clippy --all-targets -- -D
  warnings` are clean

## 4. Live

- [ ] 4.1 A real session — verify: with a request that ends while a long question is
  scrolling, the line visibly breaks into particles and is pulled out the side it was
  heading for; a short question does the same to the left; typing in the middle of it
  clears it immediately
