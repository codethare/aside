# Design

## One multiplier, and the reason the chrome follows it

The panel has two ways of being big today:

- `self.scale`, the output's integer scale, which multiplies the surface, `PAD`,
  `SEPARATION`, the radius and the shadow, and which the font is loaded at;
- the configured font size, which multiplies the glyphs **only** — `scale_of(font)`
  divides the loaded size by the configured one, so a bigger font leaves the chrome
  where it was.

Zoom is the second one made complete: the loaded size becomes
`font::size() * output_scale * zoom`, which makes `scale_of(font)` equal to
`output_scale * zoom`, and every consumer of that number — `PAD`, the gap, the
radius, the shadow, `row_h`, the bar's height, the width text is laid out at —
scales with it. That is the whole mechanism, and it is why the change is mostly
about *plumbing a number* rather than about drawing anything new.

The surface's logical size becomes `(WIDTH * zoom, whatever the phase asks for)`,
so the panel widens as well as growing: a magnified copy, not a narrower column of
bigger text.

## The places the zoom has to reach

Everything that today reads the output scale, or the bar's height, or `WIDTH`:

| Name | Today | With zoom |
| --- | --- | --- |
| `layout_width(scale)` | `answer_width(WIDTH * scale, scale)` | `s = scale * zoom` |
| `buffer_width` / `buffer_height` | `surface_size(...) * scale` | `* s` |
| `panel::bar_height()` | `row_h_for(font::size())` | `row_h_for(effective_size)` |
| `size_for(phase, output_h)` | `WIDTH` | `WIDTH * zoom` |
| `room_for_replies`, `reply_cap`, `edge_gap`, `bottom_margin` | logical px | unchanged, because the bar and the panel they compare against are already zoomed |
| `panel::draw(..., scale)` | `scale` | `s` |
| `panel::transcript_px(h, font, scale)` | `scale` | `s` |

The rule for the implementation: **`s` is the only multiplier the drawing code
sees**, and it is computed once per change as `scale * zoom` rounded to at least 1.
Nothing takes `zoom` itself except the code that computes `s` and the surface size.

## Clamping

Two limits, in this order:

1. **Range**: `zoom` stays within `MIN_ZOOM ..= MAX_ZOOM`. The keyboard steps by a
   fixed factor; the wheel by a smaller one; a pinch multiplies by the gesture's own
   scale, so a fast pinch cannot jump the range.
2. **The output**: the panel must fit. Its width becomes `WIDTH * zoom` logical, and
   the height is already capped by `room_for_replies(output_h)`; zoom is reduced (not
   the panel) until both fit the output the surface is on, with `EDGE_MARGIN`
   respected. Zooming in on a screen too small to hold it therefore stops rather than
   clipping.

## The three inputs, one function

```rust
fn zoom_by(&mut self, factor: f32)   // clamp, load the font, rewrap, apply_size,
                                     // clamp the viewport, report the caret
```

- `Ctrl+Alt+-` / `Ctrl+Alt++` (`minus`, and `plus` or `equal` — `+` is Shift+`=` on
  most layouts) call it with `1 / STEP` and `STEP`, and are matched in `key_action`
  beside the `Alt+hjkl` chords, so every mode has them.
- `Ctrl` + the wheel: `PointerEventKind::Axis` when `self.modifiers.ctrl` is set calls
  it with `1 / WHEEL_STEP` or `WHEEL_STEP` per notch. The wheel over the replies
  keeps scrolling when `Ctrl` is not held, which is today's behaviour.
- Pinch: `zwp_pointer_gesture_pinch_v1.scale` is already a *relative* factor, so the
  handler calls `zoom_by(zoom * scale_from_event)` — or, to keep one entry point,
  `zoom_to(factor * current)` is expressed as `zoom_by(event_scale)`.

A pinch that is `cancelled` leaves the zoom where it got to, which is what a reader
expects from a gesture that was going the wrong way.

## What is not animated

A zoom step is instant. Animating it would mean animating the font metrics of every
row and the surface size together, and the panel's existing morphs animate *shapes*,
not metrics; the frame after a keypress is the new zoom in full.

## Tests

- `zoom_by` steps and clamps: `zoom_step` is a pure function over `(current, up)`,
  and its range ends are the `SIZE_RANGE` ones the configuration already accepts.
- The wheel with `Ctrl` changes the zoom and leaves the scroll position alone; the
  wheel without `Ctrl` still scrolls. Both against a fake `Axis` event.
- A pinch's `scale` events move the zoom by their product, and a `cancelled` one
  stops it.
- One drawing test: the same scene at 1.0 and at 1.5 has a bar 1.5× as tall and a
  card 1.5× as wide, and the same number of rows (a magnified copy).
- The panel never exceeds the output at the highest zoom the output allows.
