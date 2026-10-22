# Tasks

## 1. Chrome: `wlmenu`'s current numbers

- [x] 1.1 Port the signed-distance shadow for a list of cards and adopt `SHADOW` 20 with the layers `(6, 155, 1)` and `(13, 115, 4)` — verify: a test asserts the margin fits the widest layer, a shadow pixel outside a card's corner exists, and no shadow is painted under a card's body
- [x] 1.2 Adopt the lit and dark hairline profiles and the plush inks — verify: a pixel test asserts the top rim is lit, the bottom rim is dark, and the palette test still clears WCAG AA with the ink composited over the body
- [x] 1.3 Add `SEPARATION` (12) and draw two cards — verify: a test finds the two card outlines with exactly 12 px of shadow-only space between them

## 2. One tone, no rail, ink highlight

- [x] 2.1 Delete `BG_CARD`, `rounded_rect` and the per-block padding; paint both cards and the ball in `BG_PROMPT` — verify: a test samples the bar row, the reply row and a gap and finds one tone for the cards and neither body's tone in the gap
- [x] 2.2 Remove the bar's 4 px `LABEL_ACCENT` rail — verify: the rail test is gone and no pixel in the bar carries `LABEL_ACCENT`
- [x] 2.3 Mark the highlighted line by ink: `FG_SEL` text, 2 px inset, no mark in the margin — verify: a test asserts the highlight changes no pixel left of the text column and no row's position, and that the highlighted line's ink differs from its neighbours'

## 3. Position and reveal

- [x] 3.1 Place the panel's bottom edge 38 % of the output height above the screen's bottom, and derive the height cap from the same margin — verify: `size_for` and the margin function agree for 1080, 1440 and a tiny output
- [x] 3.2 Add the reveal: 160 ms ease-out from the bar, panel size committed once, a 16 ms wake while it runs — verify: a test asserts `reveal_at(0) == 0`, `reveal_at(160) == 1`, that it is monotonic, and that it starts only when the replies card goes from absent to present
- [x] 3.3 Clip the card's body, hairline, shadow and rows to the revealed height — verify: a test paints the frame at half the reveal and finds no card pixel above the revealed height, with the bar unchanged

## 4. Docs

- [x] 4.1 Update `README.md`, `docs/aside.1`, `docs/live-checklist.md` and `CLAUDE.md` rule 4 for the new chrome numbers, the two same-tone cards, the ink highlight, the 38 % position and the reveal — verify: each document says all five, and none still mentions a rail, `BG_CARD` or a 24 px shadow margin

## 5. Integration

- [x] 5.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [x] 5.2 Render `aside --preview out.ppm` and look at it — verify: two same-tone cards on plush's shadow, no blue line anywhere, the replies above the bar
- [ ] 5.3 Run a real session — verify: the panel sits just below the vertical centre, the reply card rises out of the bar on the first answer and not on a follow-up, and the highlight is legible without a rail
