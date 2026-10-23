# Tasks

## 1. The caret is the input's caret

- [x] 1.1 `draw_bar` splits the input at `input.caret()` and paints before, pre-edit, after, with the caret between them
- [x] 1.2 The label, the hint and the mode tone are untouched by the split

## 2. A long line scrolls

- [x] 2.1 `scroll_to` picks the character boundary and the sub-character shift that put `scroll` px off the left edge
- [x] 2.2 The scroll keeps the caret inside the bar; nothing caps the input's length in the painter

## 3. The line forms with the bar

- [x] 3.1 `draw_text` takes the rows it may paint on; `blit_plane` skips anything outside them
- [x] 3.2 `draw_bar` lays the line out at `font.row_h` from the card's bottom and clips to the card's rows; the caret and the pre-edit underline use the same clip

## 4. Tests, docs and gate

- [x] 4.1 A test per defect: the caret column follows `Left`/`Right`, a long line keeps its caret and scrolls home, and no line is painted above the unfolded card
- [x] 4.2 README and man page say the line scrolls and the caret can be moved; the live checklist checks all three by hand
- [x] 4.3 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`, `--preview`
