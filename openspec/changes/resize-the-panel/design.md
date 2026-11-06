# Design

## The box is resized; nothing is scaled

`zoom` was the wrong word for it and the wrong behaviour: it multiplied the whole
panel, glyphs included. A resize multiplies two things only:

- the panel's width (`box_width(size) = WIDTH * size`);
- the room the replies are given (`reply_cap` scales the share, then clamps it to the
  room the output actually has).

Everything else keeps its own size: the face is read at `font::size() * output_scale`,
so the glyphs, `row_h` and the bar's height are what they were; `PAD`,
`SEPARATION`, `CORNER_RADIUS` and `SHADOW` are constants, not functions of the box.

## The adaptive part is the wrap

The text width comes from the panel's own width, not from `WIDTH`:

```rust
fn text_width(&self) -> f32 { layout_width(self.width, self.s()) }
```

`apply_size` sees the width change and rewraps the transcript once (a wrap is a
property of the width). Nothing else has to know: the viewport is clamped, the
caret's rectangle is reported again, and the panel redraws.

## Clamps

1. The range: 75–200 % of the design box, so a keypress can always come back.
2. The output: the replies' room stops where the screen's room above the bar does,
   and the width stops at the output's width when the panel is placed. The box never
   leaves the output.

## Failure modes

- No trackpad gesture on a compositor that does not send one: the pinch simply never
  arrives, and the chords and the wheel are the feature.
- A face that cannot be read is no longer a failure path here: a resize does not read
  the font at all, which it did while this was a zoom.
