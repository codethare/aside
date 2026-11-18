# Tasks

## 1. A missing character is visible

- [x] 1.1 Measure and draw agree: a character with no chain glyph advances by the
  space's width and draws a hollow box — verify: a test draws `a<U+10FFFD>b` and
  sees ink between the letters, and the drawn pen ends where `advance` says
- [x] 1.2 The box is clipped to the row range like glyphs are — verify: a test draws
  a missing box with an empty row range and asserts no pixel changed while the pen
  still advanced

## 2. A tab keeps its width

- [x] 2.1 `\t` advances four space widths in both `measure` and `draw_text`, and inks
  nothing — verify: a test asserts the width of `"\t"` is four spaces and that its
  frame is byte-identical to an empty line, with the pen moved

## 3. Scripts outside the chain can arrive

- [x] 3.1 The chain is growable and a first missed glyph appends the system's script
  families (Devanagari, Thai, Bengali, Tamil, Telugu, Kannada, Malayalam, Gujarati,
  Gurmukhi, Oriya, Sinhala, Lao, Khmer, Myanmar, Ethiopic, Hebrew, Arabic) and emoji,
  deduplicated, once — verify: a test misses on a private-use character, asserts the
  fallback runs once, and asserts no chain entry repeats
- [x] 3.2 The database scan stays off startup — verify: the existing "CJK face is not
  read before a CJK glyph" test still passes, and the scan is reached from `glyph`,
  not `load_with`

## 4. Integration

- [x] 4.1 Run the gate — verify: 259 tests pass, only the four pre-existing
  font-environment failures fail, `cargo fmt --check` and `cargo clippy --all-targets
  -- -D warnings` are clean
- [ ] 4.2 Prove a Devanagari or Thai glyph on a machine with that Noto face — verify:
  the character has ink and its advance is non-zero (not available in this sandbox)
