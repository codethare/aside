# Tasks

Each task is RED first: the named test is written and run before the code that makes
it pass, and the run is read. Gate per task: `cargo test <name>` green, and
`cargo test` green at the end of the task. One commit for the change at the end, as
this repo does for an openspec change.

## 1. The buffer

- [ ] 1.1 `src/editor.rs`: `Buffer { lines, row, col, top, cmd, pending, mode }`, `Mode { Insert, Normal }`, `CAP = 16 * 1024`, `VISIBLE = 8`; `from_text` / `text` round-trip through `\n`
- [ ] 1.2 Insert-side edits: `insert`, `newline`, `backspace` (joining lines at column 0), `delete`, and the cap refusing to grow
- [ ] 1.3 Motions by character: `left`, `right`, `up`, `down`, `home`, `end`, `word_left`, `word_right`; `scroll` keeps the caret inside `top..top + VISIBLE`
- [ ] 1.4 Tests: `the_buffer_round_trips_lines_and_keeps_the_caret`, `enter_inserts_a_line_break_and_backspace_joins_it`, `the_caret_moves_by_characters_not_bytes` (CJK), `the_cap_refuses_to_grow`, `a_motion_stops_at_the_ends_of_the_buffer`

## 2. Normal mode

- [ ] 2.1 `normal_key(keysym) -> Option<NormalKey>` and the actions: `x`, `dd`, `i`, `a`, `A`, `I`, `o`, `O`, `gg`, `G`
- [ ] 2.2 `pending` holds `g` or `d` for one key and any other key clears it
- [ ] 2.3 Tests: `dd_and_gg_wait_for_their_second_key`, `dd_on_the_last_line_leaves_one_empty_line`, `x_at_the_end_of_a_line_takes_a_character_not_the_newline`, `i_and_a_enter_insert_on_their_own_sides`

## 3. The command line

- [ ] 3.1 `start_command`, `command_key` (`Char`, `Backspace`, `Escape`, `Enter`), `resolve` returning `Write` for `wq`/`q`/`x`, `Discard` for `q!`, `Unknown` otherwise
- [ ] 3.2 Tests: `a_command_line_answers_what_it_means`, `an_unknown_command_keeps_the_line_and_says_so`, `escape_closes_the_command_line_and_stays_in_normal`

## 4. The bar and the surface

- [ ] 4.1 `Buffer::rows()` = `min(lines, VISIBLE)` plus one while the command line is open; `panel::bar_height()` unchanged for one row
- [ ] 4.2 `size_for(phase, output_h, editor_rows)`; `apply_size` on entering and leaving; the bottom edge never moves
- [ ] 4.3 Tests: `the_bar_grows_by_the_editor_rows_and_shrinks_back`, `the_surface_keeps_its_bottom_edge`, `the_replies_card_is_untouched_by_the_editor`, `a_short_output_still_fits_one_editor_row`

## 5. Painting

- [ ] 5.1 `panel::Editing { lines, caret, label, command }` passed to `panel::draw`; the bar card is `row_h * rows` tall
- [ ] 5.2 One loop over the visible rows: text per line, the caret a bar at its row and column, the horizontal scroll reused per line, the command line on its own row, the mode label on the right
- [ ] 5.3 Tests: `the_editor_paints_the_caret_on_its_own_row`, `the_command_line_is_a_row_of_its_own`, `a_long_line_scrolls_under_the_caret`, `the_label_names_the_editor_mode`

## 6. Wiring

- [ ] 6.1 `Mode::Editor`; `Ctrl+G` in typing opens it on `input.text()`; `Ctrl+G`, `Esc` in normal and a `Write` verdict all leave and put the text back; a `Discard` verdict leaves the input line untouched
- [ ] 6.2 The IME is enabled in insert and disabled in normal; `report_cursor` reports the caret's row and column
- [ ] 6.3 `Alt+hjkl` chords still move the panel; `Ctrl+V` still pastes
- [ ] 6.4 Tests: `wq_puts_the_text_back_in_the_input_line`, `q_bang_drops_what_was_typed`, `the_ime_is_on_in_insert_and_off_in_normal`, `the_window_chords_work_in_the_editor`, `a_sent_question_carries_its_newlines`

## 7. Docs and gate

- [ ] 7.1 `README`, `docs/aside.1`, `CLAUDE.md` rules 1 and 3, `docs/live-checklist.md`
- [ ] 7.2 `--preview` shows the editor open on a four-line question
- [ ] 7.3 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`, `cargo audit`
