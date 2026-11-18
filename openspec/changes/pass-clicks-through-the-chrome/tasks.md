# Tasks

## 1. Only the cards take input

- [x] 1.1 The input region is the replies card plus the bar, in surface coordinates —
  verify: a pure test pins both rectangles, the 16 px gap between their edges, and the
  shadow margin outside them
- [x] 1.2 The region is set on the final layer surface and recomputed whenever
  `apply_size` changes the card geometry, including the idle bar — verify: the region
  is built from the same `width`/`height` the painter uses, and the protocol object is
  destroyed after the surface has taken it

## 2. Integration

- [x] 2.1 Run the gate — verify: 266 tests pass, only the three pre-existing
  font-environment failures fail, `cargo fmt --check` and `cargo clippy --all-targets
  -- -D warnings` are clean
- [ ] 2.2 A real session — verify: click a window visible in the shadow margin and in
  the gap between the cards; it becomes focused and typable, and the panel does not
  move. Click the cards themselves; the panel still takes it
