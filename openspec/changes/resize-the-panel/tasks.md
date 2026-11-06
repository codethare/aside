# Tasks

- [x] 1.1 `size_step`, `size_chord`, `wheel_size` as pure functions, clamped to `SIZE_RANGE`
- [x] 1.2 Tests: the step comes back, the range stops it, the chords need both modifiers, a wheel notch counts, a touchpad is one step
- [x] 1.3 `State::size`, `resize_by`/`set_size`: the width and the replies' room change, the face is not read again
- [x] 1.4 `size_for`, `reply_cap`, `room_for_replies`, `box_width`: the box grows and stops at the output
- [x] 1.5 `layout_width(width, scale)` and `text_width()` from the panel's own width: the text wraps at the new edge
- [x] 2.1 `Ctrl+Alt+-`/`+` (and the keypad), `Ctrl+Alt+BackSpace`, matched before the mode branches
- [x] 2.2 `Ctrl` + the wheel resizes; without `Ctrl` it still scrolls
- [x] 2.3 `zwp_pointer_gestures_v1` bound by hand, one pinch per seat, `update` scales the box, `cancel` leaves it
- [x] 3.1 Test: the bar and the glyphs keep their size, the width and the room grow, the wrap is wider, the box stays on the output
- [x] 1.6 `size_cap(output_h, output_w)` and `clamped_size`: the factor stops at the
      first direction that would leave the output, and `set_size` goes through both
- [x] 1.7 Test: at the cap one side is flush with the output, past it the factor has
      stopped meaning anything on that side, and a wider or taller output moves it
- [x] 4.1 `README`, `docs/aside.1`, `docs/live-checklist.md`, `CLAUDE.md`
- [x] 4.2 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`, `--preview`
