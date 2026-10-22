# Tasks

## 1. The wheel scrolls the replies

- [x] 1.1 Add `Viewport::scroll_by` with the follow rule — verify: layout tests scroll up (following stops), scroll past the bottom (clamps and follows), and scroll a transcript shorter than the window (no movement)
- [x] 1.2 Handle axis events: three lines per notch, pixels for a continuous delta, only while the pointer is over the replies card — verify: a test asserts the pointer's region check, and a real session scrolls with a wheel and with two fingers

## 2. The prefix bar

- [x] 2.1 Give `render::body` its tone back and add the mode tone — verify: the palette test still clears AA with the hint's ink on it
- [x] 2.2 Draw the hint in place of the input, with no caret, while the prefix waits — verify: a panel test finds the hint's ink, no input ink, no caret and the mode tone in the bar; the typing frame is unchanged
- [x] 2.3 Make sure the prefix cannot take text and that leaving restores everything — verify: a test asserts a letter pressed in prefix mode reaches no input line, and that the typing frame after leaving is byte-identical to before entering

## 3. Docs

- [x] 3.1 Update `README.md`, `docs/aside.1` and `docs/live-checklist.md` for the wheel and the prefix bar — verify: each names both, and neither says the prefix is invisible

## 4. Integration

- [x] 4.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [ ] 4.2 Run a real session — verify: the wheel and a two-finger slide scroll the replies, nothing scrolls over the bar, `Ctrl+T` turns the bar and shows the hint, and no key types until the prefix ends
