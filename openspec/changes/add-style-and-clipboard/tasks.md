# Tasks

## 1. The keyboard is not taken away

- [x] 1.1 Ask for on-demand interactivity instead of exclusive — verify: with the panel open, clicking another window focuses it and typed text goes there, and clicking back returns typing to the panel

## 2. The clipboard both ways

- [x] 2.1 `clipboard::paste` reads the selection with `wl-paste --no-newline` — verify: a stub `wl-paste` puts its output in the input line, and a missing `wl-paste` is reported in the transcript rather than swallowed
- [x] 2.2 `Ctrl+C` copies and `Ctrl+V` pastes, `Alt+C` unchanged — verify: the key-table test covers all three, and the paste test inserts at the caret with the panel's existing editing keys working afterwards

## 3. The font comes from the file

- [x] 3.1 `font_size` and `font_family` are accepted, validated (8..=72) and resolved before the window opens — verify: a size outside the range is one stderr line naming file, line and range with a non-zero exit; a valid size and family reach the loaded face
- [x] 3.2 The bar, the row height, the reply box and MAX_ROWS follow the configured size — verify: a test loads 30 px and finds `bar_height()` = `row_h_for(30)` and the panel taller by that difference
- [x] 3.3 A family nothing provides falls back to the built-in chain — verify: a test loads a nonsense family and still gets a usable font with CJK covered

## 4. Docs

- [x] 4.1 `README.md`, `docs/aside.1`, `docs/live-checklist.md` and `CLAUDE.md` name the two config keys, the two clipboard keys, the on-demand focus and the new `wl-paste` program — verify: each says all of them, and the security section no longer implies the panel holds exclusive focus

## 5. Integration

- [x] 5.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [ ] 5.2 Run a real session — verify: font size and family from the file are honoured, `Ctrl+V` pastes, `Ctrl+C` copies, and clicking a window behind the panel works
