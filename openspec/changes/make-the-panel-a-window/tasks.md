# Tasks

## 1. The surface

- [x] 1.1 `XdgShell::create_window` with `WindowDecorations::None`, `app_id` and title `aside`; the buffer-less first commit asks for the configure
- [x] 1.2 `WindowHandler::configure` in place of `LayerShellHandler::configure`; `request_close` sets the exit flag
- [x] 1.3 `apply_size` keeps the size and drops every layer-shell call; the state field is `window`

## 2. Nothing moves the panel any more

- [x] 2.1 Delete `Key::Window`, `window_move`, `WINDOW_STEP`, the `Alt+hjkl` branches of `key_action` and the chord block in `on_copy_key`
- [x] 2.2 Delete `Drag`, `dragging`, `drag_to`, `move_panel`, `margins`, `left_margin`, `base_gap`, `edge_gap`, `bottom_margin`, `dragged_left`, `dragged_bottom`, `drag_for_bottom`, and `Motion::Drag`
- [x] 2.3 The pointer still selects: press on a line anchors, motion with the button down extends, hover does nothing

## 3. Size

- [x] 3.1 `room_for_replies` is 38 % of the output height capped at 64 rows; `reply_cap`, `REPLY_SHARE` and `EDGE_SHARE` go

## 4. Tests, docs and gate

- [x] 4.1 Tests: the box is a share of the output and capped, nothing moves the panel, and no chord is a panel move
- [x] 4.2 README (float rules for sway, niri, river; the chord and drag gone), man page, live checklist, `CLAUDE.md` rule 3 and rule 4
- [x] 4.3 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`, `--preview`
