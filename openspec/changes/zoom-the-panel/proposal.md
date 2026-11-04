# Zoom the panel with the keyboard, the wheel and a pinch

## Why

The panel has one size. `Ctrl+T` then `+`/`-` does change the *font* by two points,
but nothing else moves with it: the padding, the corner radius, the shadow, the gap
between the cards and the panel's width all stay where they were, and the box holds
fewer characters the bigger the text gets. What a reader means by "make it bigger"
is the whole thing, and what a trackpad or a wheel already offers is a gesture for
it.

## What changes

One **zoom** multiplier on the panel, from 1.0, with three ways to move it:

- **Keyboard**: `Ctrl+Alt+-` smaller, `Ctrl+Alt++` bigger, in every mode — the same
  "every mode" rule `Alt+hjkl` follows, so reading a long answer in copy mode does
  not take the zoom keys away.
- **Wheel**: `Ctrl` + the wheel over the panel, one notch per step.
- **Pinch**: two fingers on the trackpad while the pointer is on the panel, which is
  the `zwp_pointer_gesture_pinch_v1` gesture, continuous rather than stepped.

Everything scales together, which is what makes it a zoom rather than a font change:
the glyphs, `PAD`, `SEPARATION`, the corner radius, the shadow margin, the bar, the
row height, the panel's width. Nothing is written anywhere: the zoom lives as long
as the process, exactly like the position it moves.

## What it costs

- `zwp_pointer_gestures_v1` is **not** wrapped by `smithay-client-toolkit` 0.21 (it
  has pointers, keyboards, touch and relative pointers, no gestures), so the pinch is
  the only part that is new Wayland plumbing: bind the global, make one
  `zwp_pointer_gesture_pinch_v1` for the seat, handle `scale` and `cancelled` events,
  and add the two `Dispatch` arms. `wayland-protocols`' `unstable` feature is already
  on, so no new dependency.
- A zoom step is a font read (a few milliseconds on the event loop, as the existing
  `rescale` already accepts), a re-wrap and one surface resize.
- The panel can outgrow its output, so zoom is clamped: by range, and by what the
  output can hold.

## Impact

- `src/main.rs`: the `zoom` field, one `zoom_by`, the two chords, the wheel arm, the
  pinch bind and handler, and the logical-size functions that have to see the zoom.
- `src/font.rs`: the loaded size becomes `configured * zoom * output scale`, so the
  chrome that is derived from the font's size scales with it.
- `README`, `docs/aside.1`, `docs/live-checklist.md`, `CLAUDE.md`.
- No dependency, nothing written to disk.
