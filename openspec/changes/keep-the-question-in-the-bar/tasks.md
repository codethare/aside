# Tasks

- [x] 1.1 `marquee(elapsed, travel, s)` and its constants: the far end, the hold, the
      turn, and a line that fits never moving
- [x] 1.2 Tests: it starts at nothing, reaches the end, holds there, comes back, and
      stays inside `0..=travel` over a sweep
- [x] 2.1 `panel::Pending { text, elapsed }`; `draw`/`draw_bar` take it
- [x] 2.2 The bar draws the question at the marquee offset, with no caret and no
      pre-edit; a question that fits sits still at the left edge
- [x] 2.3 Test (pixels): the first character is on screen at age zero, the ink moves
      by a later age, and no caret band is painted either time
- [x] 3.1 `State::question` set in `send` with the instant it went out
- [x] 3.2 Dropped when the stream ends, on cancel, on a refusal, and when a key reaches
      the line; shown only in typing mode
- [x] 3.3 Test: the bar shows it in typing mode only
- [x] 4.1 `README`, `docs/aside.1`, `docs/live-checklist.md`, `CLAUDE.md`
- [x] 4.2 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
      `groff -man -Tutf8 docs/aside.1`, `--preview`
