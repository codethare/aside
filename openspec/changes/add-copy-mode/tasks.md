# Tasks

## 1. The window moves by chord

- [x] 1.1 `Alt+hjkl` moves the panel 16 px and `Alt+Ctrl+hjkl` 160 px, in every mode — verify: the key table maps those eight chords and no other Alt chord to a window move, and a press moves by exactly that many pixels

## 2. Copy mode

- [x] 2.1 Delete `Mode::Vim`, `vim_move`, `on_vim_key`, `enter_vim`, `leave_vim` and the `-- VIM --` label; `[` in the prefix enters `Mode::Copy` — verify: the prefix's hint names copy mode, and no key reaches the input line in copy mode
- [x] 2.2 Character, line, line-edge, page, half-page and first/last motion move a cursor with the pointer's own row/character maps — verify: a pure test covers movement and its clamping at both ends of a line and of the transcript
- [x] 2.3 `w`/`b` walk words across lines, breaking where the wrapper breaks — verify: a test walks a Latin line and a CJK line and lands on the words it should, including across a line boundary
- [x] 2.4 `Space`/`v` start a selection and motion extends it, drawn as the same band the pointer draws — verify: a frame test finds the band and the cursor in one frame
- [x] 2.5 `Enter`/`y` copy the selection (or the cursor's line) and leave; `q` leaves; `Esc` clears first — verify: three tests, one per exit, and the clipboard contents checked against a stub
- [x] 2.6 The bar reads `-- COPY --`, then `copied N lines` for about a second — verify: a test drives the label through both, and the bar's geometry is unchanged

## 3. Docs

- [x] 3.1 `README.md`, `docs/aside.1`, `docs/live-checklist.md`, `CLAUDE.md` rule 3 — verify: each names the window chords and copy mode, and none mentions vim mode

## 4. Integration

- [x] 4.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [ ] 4.2 Run a real session — verify: `Alt+hjkl` moves the panel from every mode, `Ctrl+T [` walks the replies, `w`/`b` move by words, and `Enter` copies and leaves
