# Tasks

## 1. The size is a value, not a constant

- [x] 1.1 `font::configure` stops being a `OnceLock`: the configured size can be read and replaced while the process runs
- [x] 1.2 `font_step(current, up)` steps two pixels and clamps to `SIZE_RANGE`, with a test for both ends
- [x] 1.3 `Prefix::Bigger` / `Prefix::Smaller` are in the prefix table (`+`, `=`, `-`), and only there; a test covers the keys and that typing still types them

## 2. The panel follows the new size

- [x] 2.1 The font is read again at the new size, the transcript is laid out again and the surface is resized once through `apply_size`
- [x] 2.2 The prefix hint names the keys, and the key table test still pins the one-key-wide prefix
- [x] 2.3 A test covers the re-derivation: the bar height and the row height follow the size
- [x] 2.4 A resize re-reports the caret, so the input method's candidate window follows the bar instead of landing on the old row

## 3. Docs and gate

- [x] 3.1 README and the man page list the prefix keys, the range and the step
- [ ] 3.2 A hand check: `Ctrl+T +` a few times up to the limit and back down, with an answer on screen
- [x] 3.3 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`
