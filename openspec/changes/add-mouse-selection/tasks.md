# Tasks

## 1. A drag that starts at once

- [x] 1.1 Delete the hold and start the drag on the first motion of a press that did not begin on a reply line — verify: a press-and-move carries the panel from the first motion event, and a press released without motion moves nothing
- [x] 1.2 Record the press position on press, so the first motion has a delta — verify: the drag's first step equals the pointer's own step, not the distance travelled while waiting

## 2. Selecting reply lines

- [x] 2.1 `Transcript::row_at` maps a transcript y to a row — verify: a test walks every row and finds `row_at(row_y(i)) == i` through a wrapped, multi-block turn, and out-of-range y gives `None`
- [x] 2.2 Press on a reply line anchors a selection, motion extends it, release keeps it, a press elsewhere moves the panel — verify: a pure test covers anchor/extend/normalise, and that a press on the bar never anchors
- [x] 2.3 Rows in the selection are painted in the highlight ink — verify: a frame test finds the selected rows inked and their neighbours not
- [x] 2.4 Copy takes the selection, one line per line; `Esc` clears it before closing and a new question clears it — verify: copy of three lines yields exactly those three, `Esc` clears then closes, and sending clears

## 3. Docs

- [x] 3.1 `README.md`, `docs/aside.1`, `docs/live-checklist.md` — verify: each says the drag follows the pointer at once and that dragging selects lines, and `Alt+C`/`Ctrl+C` copy the selection

## 4. Integration

- [x] 4.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [ ] 4.2 Run a real session — verify: dragging the bar moves the panel immediately; dragging over reply lines selects them; `Ctrl+C` pastes them elsewhere as separate lines
