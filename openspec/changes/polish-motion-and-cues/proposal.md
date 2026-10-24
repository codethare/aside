# Polish the motion and the cues

## Why

The panel's motion was written to be seen, and three of its cues stay switched on
when there is nothing to see. Checked against the Spotlight breakdown — the design
reference for this change — five things differ, and all five are a timing or a
threshold, never a shape:

- The shape change is an under-damped spring clamped at its target, so it is at the
  finished shape 31 % of the way through its run and holds there for the remaining
  250 ms. The one thing the eye reads as "the panel grew" is over in a third of the
  time the code says it takes.
- Closing runs as long as opening (460 ms against 520 ms). An exit should not be as
  slow as an arrival.
- The working light is up from the first frame of a request, so a fast answer gets a
  flash of it and nothing else.
- The 3 px scroll indicator is permanent whenever the transcript overflows, in the
  accent colour, at rest.
- Replies are cut off flat at the bottom edge of their card, mid-row.

The reference asks for the opposite of each: no overshoot, an exit shorter than the
entrance, a spinner only for slow queries, an indicator only while the view moves,
and content that dissolves where the card ends. What it does not ask for, and what
this change does not take, is its geometry: the bar stays 640 px wide and
bottom-anchored, and the replies still grow upward out of it. The reference's own
last section rules its own geometry out for `aside`.

## What changes

Five, each one constant or one predicate:

1. **No overshoot.** `morph_at` becomes the same ease-out the entrance already uses
   (`track_at`): monotone, exactly 1 at the deadline, never past it. The spring, its
   damping, its frequency and its re-normalisation go.
2. **A shorter exit.** `CLOSE_MS` 460 → 200 ms, with the closing still the opening
   run backwards, tracks and all. Only the duration changes; the line and the point
   stay.
3. **A working light only for slow requests.** `LIGHT_AFTER_MS = 500`: the rim stays
   dark for the first half second of a request, and the lap starts from that moment
   rather than from the keypress. The loop sleeps to the threshold instead of
   repainting a frame that cannot change.
4. **A scroll indicator only while the view moves.** It appears on a wheel notch or
   a page key, holds at full ink for 700 ms and fades out over the last 200 ms.
5. **A dissolve at the replies' bottom edge.** The last row of the replies card fades
   its text out instead of cutting it, so a line leaving the view leaves rather than
   ends. One row tall, so a line that fits is never dimmed.

## Trade

The bar is silent between `Enter` and the light, which is the point of the delay: a
request answered in under half a second now shows no working indication at all. The
key table is unaffected, `Esc` still cancels, and the light comes up at 500 ms for
everything slower, which is where a person starts to wonder.

A shorter exit means the collapse to a line is quicker than the spread out of a
point. That asymmetry is the reference's rule (out faster than in) and it keeps the
close mirrored, which a fade would not.

## Rejected

- **The reference's fade-out exit (100 ms, no scale).** It deletes the collapse into a
  line and the test that guards it, and it leaves an arrival that grows out of a
  point with no matching departure.
- **A grey indicator, as Spotlight has.** `LABEL_ACCENT` is a `plush` token and the
  panel's own; the change is when it shows, not what colour it is.
- **A per-pixel dissolve.** Per-row is enough at this ramp width and keeps the frame
  cost per visible row, which the spec already requires.
- **Keeping the spring and only dropping the clamp.** That would be a real 7 %
  overshoot, which is the one thing the reference rules out by name.
