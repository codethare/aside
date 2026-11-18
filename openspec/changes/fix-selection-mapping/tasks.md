# Tasks

## 1. The pointer names the character the band covers

- [x] 1.1 `render::char_at` returns a character count, not a byte offset — verify:
  a test uses a two-byte `é`: inside it names 1, past the line names 2
- [x] 1.2 `reply_spot` maps x in buffer pixels: scale, card inset, row indent and the
  keyboard's 2 px nudge — verify: a pure test pins 1×, 2× with a 10 px indent, and
  the inked row
- [x] 1.3 `over_replies` takes the panel's own height and the whole replies card is
  selectable — verify: the region test's geometry includes the shadow margin, and
  a press in the card's last rows is a selection, not a panel drag

## 2. Indices survive only while the text does

- [x] 2.1 A re-wrap (width) and a scale change clear the pointer selection — verify:
  the code path is shared with the existing resize tests; a selection cannot be
  indexed after the rows moved

## 3. Copy says what it did

- [x] 3.1 `copy_highlight` returns whether text reached `wl-copy`; copy mode only
  notes `copied N lines` when it did — verify: an empty selection no longer sets the
  note; the failure path still shows its error
- [x] 3.2 `Alt+C`/`Ctrl+C` show the copied-lines note too — verify: the note path is
  the same `self.copied` the bar already times
- [x] 3.3 A selection over a multibyte row slices whole characters — verify: a test
  selects `文a` from `中文abc` by character index and byte-slices it

## 4. Integration

- [x] 4.1 Run the gate — verify: 265 tests pass, only the three pre-existing
  font-environment failures fail, `cargo fmt --check` and `cargo clippy --all-targets
  -- -D warnings` are clean
- [ ] 4.2 A real session at 1× and 2× — verify: click after a CJK character, on an
  indented code line, and near the card's bottom; the band starts under the pointer,
  and `Ctrl+C` copies exactly it
