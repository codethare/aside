# Design

## Context

`aside` draws a 640 px bar anchored 38 % of the output height above the bottom, with
the replies in a second card above it. Its motion is already the shape the reference
describes: a fixed bar, one card that only grows upward, the anchor on the edge that
does not move, no fill of its own behind the transcript, and `plush`'s rim and shadow
rather than a vibrancy material. What it does not have, and what this change adds, is
the reference's judgement about *when* a cue is worth showing and how a movement
should land.

The reference is a deconstruction of Apple's Spotlight, written up from screenshots
and use. Its numbers are observed, not documented, and its geometry is deliberately
not ours. Everything taken from it here is a duration, a threshold or an edge
treatment, which is the part that transfers.

## The measured starting point

`morph_at` is `1 - e^(-5t)·cos(1.6πt)`, normalised so its value at the deadline is
exactly 1 and clamped to `[0, 1]`:

| t (share of the run) | 0.1 | 0.2 | 0.3 | 0.31 |
|---|---|---|---|---|
| value | 0.469 | 0.805 | 0.988 | **1.000** |

The curve passes 1.0 at 31 % and is clamped there for the rest of the run. So the
"spring" is not felt as a 7 % overshoot — the clamp removes it — but as an ease-out
that finishes at 140 ms of a 460 ms change and then holds a still frame for 320 ms.
The tail is dead time, not motion.

## The five changes

### 1. A shape change lands on its deadline

`morph_at` becomes `track_at` — the ease-out cubic the entrance already uses: `1 -
(1 - t)³`, monotone, 0 at the start, exactly 1 at the deadline, never past it. The
clamp, the damping, the frequency and the normalisation go with the spring.

Durations stay as they are (`MORPH_IN_MS` 360, `MORPH_OUT_MS` 460). The change is
which curve fills them, not how long they are: the panel is deliberately slow, and
the reference's 180–220 ms height expand is a result panel, not a chat card.

The test that guards it currently asserts "it arrives before the deadline", which
holds for a cubic as well. It gains one assertion — the value never exceeds 1 — so
the curve cannot become a spring again without a failure. Nothing is loosened.

### 2. The exit is shorter than the entrance

`CLOSE_MS` 460 → 200 ms. `close_tracks` and `TRACK_SHARE` are untouched: the card
rolls up into a line, the line pulls in from both ends, in 200 ms instead of 460.
Entrance to exit is then 2.6 : 1, and the two runs remain each other's reverse.

The reference's alternative — a 100 ms fade with no scale — was put to the human
partner and rejected in favour of this one, because an arrival that grows out of a
point needs a departure that goes back into one. Recorded in `proposal.md` under
Rejected.

### 3. The working light waits for a slow request

`LIGHT_AFTER_MS = 500` and a pure `light_at(elapsed) -> Option<f32>`: `None` before
the threshold, `orbit_at(elapsed - LIGHT_AFTER_MS)` after it. The lap therefore
starts at the rim's own start when the light appears, rather than 36 % of the way
round, which is what reading `orbit_at(phase_start)` would have drawn.

`State::head` calls `light_at`, so the painter is unchanged: it already takes an
`Option<f32>` and paints nothing for `None`.

The loop's `Thinking` branch stops asking for a 16 ms frame before the threshold:
it sleeps to the moment the light comes up (capped at 250 ms per wake, so a second
launch still lands promptly). After it, 16 ms frames as before.

Consequence, accepted: a request answered in under 500 ms shows no working cue at
all. That is the reference's rule — a spinner that appears and vanishes inside a
frame is noise — and the reply card growing is itself the cue from that moment on.

### 4. The scroll indicator shows while the view moves

A field `thumb_at: Option<Instant>` is set wherever the *user* moves the view: a
wheel notch or a page key. The answer growing at the tail is not a scroll, so it
does not raise the indicator.

A pure `thumb_alpha(elapsed) -> f32` gives 1.0 until `THUMB_HOLD_MS` (700) and then
falls linearly to 0 over `THUMB_FADE_MS` (200). `panel::draw` takes that alpha as one
more argument and paints the same 3 px `LABEL_ACCENT` rectangle through
`render::dim`. The geometry, the colour and the position are unchanged.

The loop sleeps to the end of the hold when the indicator is up but still at full
ink, then asks for 16 ms frames through the fade. While holding, the frame cannot
change.

### 5. The replies dissolve at the card's bottom edge

`draw_replies` already computes each row's ink and hands it a `fade` that a morph
uses. It multiplies that by an edge factor: a row is dimmed by its distance above
`clip.end`, over a ramp one row tall — `font.row_h`, so it scales with the font and
the output and a row that fits is never dimmed.

The selection band and the copy cursor take the same factor — in `draw_row`, the
band's `BG_SEL` goes through `render::dim` like the text already does — so a
selection at the edge does not stay a hard block behind faded glyphs.

Only the bottom edge dissolves. The top edge keeps the existing rule that a row
partly scrolled out above it is not drawn at all; the bottom edge has no such rule
and never had.

Frame cost is unchanged: one multiply per visible row.

## Calls made here, not by the human partner

- **500 ms** for the light and **700 + 200 ms** for the indicator: within the
  reference's ranges, one constant each, reversible without touching a second line.
- **The requirement still called "The bar morphs into the ball and back out" is
  rewritten and retitled "The panel changes shape without overshooting".** The ball
  was removed by `replace-ball-with-orbit` and the requirement was left describing
  it; this change is the one that touches that morph, so it corrects the name and
  the body rather than adding a second, contradicting requirement beside it.
- **The `thinking` row and its "Enter lights the rim" scenario in "The panel is a
  layer surface in two sizes" are left alone.** They stay true — the light does
  begin, half a second in — and the delay belongs to the requirement about the light.

## What is left to the builder

The three thresholds and the ramp width are guesses at a desk with no compositor.
`--preview` renders a PPM frame, so the dissolve and the indicator alpha can be
checked by eye before a live session, and `docs/live-checklist.md` is where the
timings get confirmed on real hardware; a value that reads wrong there is a
one-constant change, not a redesign.
