# Tasks

## 1. Shadow

- [x] 1.1 Adopt `SHADOW` 26 and the layers `(9, 120, 2)`/`(20, 95, 6)` — verify: the margin assertion still holds, and a frame test finds shadow outside a card's corner and none under its body

## 2. Rim

- [x] 2.1 Adopt the lit and dark inks (90/90) and plush's two profiles — verify: a pixel test shows the top rim lit, the bottom rim dark, and the lit value fading out by four fifths of the height
- [x] 2.2 Draw the dark band one pixel inside the rim — verify: a pixel test finds the band at the bottom inside the rim and nothing at the top, on a card tall enough to show both

## 3. Gap

- [x] 3.1 `SEPARATION` 16 with `SHADOW_REACH_UP` and the assertion `SEPARATION > SHADOW_REACH_UP` — verify: a frame test measures 16 px between the cards and finds no card body inside the gap

## 4. Docs

- [x] 4.1 Update `README.md`, `docs/aside.1`, `docs/live-checklist.md` and `CLAUDE.md` rule 4 with the new margin, rim and gap — verify: each names 26 px, the rim's band and 16 px, and none still says 20 px or 12 px

## 5. Integration

- [x] 5.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [ ] 5.2 Render `aside --preview out.ppm` beside a `plush` screenshot — verify: the shadow width, rim brightness and inner band read as the same chrome
