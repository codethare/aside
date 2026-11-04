# Tasks

Numbers in brackets are [decided below] — they land when the four questions at the
bottom of `proposal.md` are answered.

## 1. One multiplier (RED first)

- [ ] 1.1 `zoom_step(current, up, step)` is a pure function: multiplies, clamps to `MIN_ZOOM..=MAX_ZOOM` [values]
- [ ] 1.2 Test: steps up and down from 1.0 and stops at both ends
- [ ] 1.3 `State::zoom: f32`; `s()` becomes `(scale * zoom).round().max(1)` and every place that reads `self.scale` for drawing, sizing or `layout_width` reads it instead

## 2. The panel grows as a whole

- [ ] 2.1 The font is loaded at `font::size() * output_scale * zoom`, so `scale_of(font)` answers the zoomed scale and the chrome derived from it follows
- [ ] 2.2 `size_for`/`bar_height` see the zoomed numbers; `room_for_replies` still caps the height
- [ ] 2.3 Test: the same scene at 1.0 and 1.5 has a bar 1.5× as tall, a card 1.5× as wide and the same row count
- [ ] 2.4 Test: the panel never exceeds the output width at any zoom the output allows

## 3. Keyboard

- [ ] 3.1 `Ctrl+Alt+minus` out, `Ctrl+Alt+equal`/`plus` in, matched beside the `Alt+hjkl` chords so every mode has them
- [ ] 3.2 Test: both chords in typing, prefix and copy mode; no text reaches the input line, no copy-mode motion happens; `Alt` alone still moves the panel

## 4. Wheel

- [ ] 4.1 `Axis` with `Ctrl` zooms by `WHEEL_STEP`; without `Ctrl` it scrolls as today
- [ ] 4.2 Test: a `Ctrl` axis event changes the zoom and leaves `viewport.top` alone; a plain axis event scrolls and leaves the zoom alone

## 5. Pinch

- [ ] 5.1 Bind `zwp_pointer_gestures_v1`, one `zwp_pointer_gesture_pinch_v1` for the seat, and the two `Dispatch` arms
- [ ] 5.2 `scale` multiplies the zoom; `cancelled` and `end` leave it where it is; a pinch with no `begin` is ignored
- [ ] 5.3 Test: the event handlers, as pure functions over `(zoom, event)` — begin, scale, cancel, end

## 6. Docs and gate

- [ ] 6.1 `README`, `docs/aside.1`, `docs/live-checklist.md` (`Ctrl+Alt+±`, `Ctrl`+wheel, pinch), `CLAUDE.md` rule 3 (the key table)
- [ ] 6.2 `--preview` at 1.0 and at a zoomed value, side by side
- [ ] 6.3 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`, `cargo audit`

## Open questions, before 1.1

1. `Ctrl+T` then `+`/`-` exists today and changes the **font size only** (the chrome
   does not follow). Merge it into the zoom — one meaning for `+`/`-` — or keep both?
2. The range and the steps: `0.75..=2.0`, keyboard `1.15`, wheel `1.05`,
   pinch continuous?
3. A key that returns to 1.0 (`Ctrl+Alt+0`)?
4. When a pinch is not available (compositor without the gesture, or a mouse), the
   wheel and the chords are the whole feature — no fallback UI beyond that?
