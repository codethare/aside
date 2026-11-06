# Resize the panel's box

## Why

The panel has one size. `Ctrl+T` then `+`/`-` changes the *text*, which is a different
thing: on a big screen a reader wants the box to be bigger — more of the answer
visible, the lines wrapped to a wider edge — not the glyphs.

## What changes

The panel's **box** has a size, from 1.0, and three ways to move it:

- `Ctrl+Alt+-` smaller and `Ctrl+Alt++` bigger, in every mode, 15 % a press;
- `Ctrl` with the wheel over the panel, 5 % a notch;
- a two-finger pinch on the trackpad, following the fingers.

`Ctrl+Alt+BackSpace` returns to the design size.

What grows is the width and the room the replies are given; the glyphs, the padding,
the corner radius, the shadow margin and the bar's height do not move. So a bigger
box shows **more text at the same size**, and the transcript is wrapped again at the
new width — the adaptive part.

The box is clamped to 75–200 % of its design size and to what the output can hold, and
the size is never written anywhere.

## Impact

- `src/main.rs`: the size field, one `resize_by`, the chords, the wheel arm, the pinch
  handler, and `size_for`/`reply_cap`/`room_for_replies`/`layout_width` taking the
  box's width.
- `README`, `docs/aside.1`, `docs/live-checklist.md`, `CLAUDE.md`.
- No dependency, nothing on disk.
