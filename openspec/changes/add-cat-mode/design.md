# Design: the cat mode

## Intent

A second way to draw the same panel, chosen in the configuration file, that gives a
reader a reason to try the thing: a black cat comes out of the screen, takes the
question on its magnified paw, licks the paw while it thinks, and answers out of its
mouth.

The mode changes what is drawn and where, never what the panel does. Every key, every
scroll, the selection, the markdown, the sandbox and the ephemeral transcript are the
same in both modes.

## Goals

- `mode = "cat"` is a complete second look; `"default"` is today's panel, byte for byte.
- The cat is drawn from the same primitives as the chrome: no asset, no new dependency,
  nothing on disk.
- The cat can come from any of the four screen edges, and its window is small enough
  that a moving cat costs less per frame than the panel does.
- The input is live in the first frame. The performance never gates typing.
- With `animations = "false"` the cat is simply there.

## Non-goals and anti-goals

- Not a general skin system: one key, two values, and no plugin shape.
- Not a cat that watches the pointer, follows the text, or reacts to anything but the
  request's phase.
- Not a cat on a second output, and not a cat in the default mode.
- Anti-goal: a mode that only looks right on one compositor. Where two surfaces meet,
  the seam is a geometric identity, not a guess.

## Constraints found in the code

- The panel is one `wlr-layer-shell` surface sized to its cards plus a 26 px shadow
  margin, anchored `BOTTOM | LEFT`, centred by `left_margin`, its bottom edge
  `EDGE_SHARE` (38 %) of the output up from the bottom. Its bar card is the bottom
  card and it never moves while the replies grow.
- Every frame repaints every pixel and damages the whole surface, because the shm slots
  rotate. A surface the size of the output would therefore repaint the output.
- One `SlotPool` can allocate buffers of more than one size, so a second surface costs a
  second buffer, not a second pool.
- The reply card is a fixed box above the bar (`room_for_replies`), and the transcript
  draws the current turn only (`Transcript::rows` from `current_start`).
- Colours are plush's: card `#2c2c2c` at 86 %, ink `#ebebf5` at 61 %, prompt ink white.

## Approaches

1. **One surface, the stage above the panel.** Grow the panel's own surface upward by a
   stage and paint the cat in it. Cheapest: no second surface lifecycle. But the cat
   can then only peek out of the panel's own edges, never the screen's, and every cat
   pixel joins the panel's frame cost.
2. **Two surfaces: the panel untouched, the cat on its own stage.** The stage is a small
   layer surface, anchored and moved by margins, carrying the cat and nothing else. Any
   of the four edges is reachable, the moving window costs 260×260 px per frame instead
   of the output's, and no line of the panel's text, scroll or selection machinery
   changes. Costs a second surface, a second buffer, and a seam that has to be exact.
3. **One surface the size of the output.** The most faithful to the description and the
   worst to run: a software repaint of the whole output for every animated frame, in a
   program whose rule is that a frame costs what the visible rows cost.

**Picked: 2.** It is the only one that gets both the four edges and a cheap frame. The
risk it concentrates is the seam, and a seam that is a computed identity (below) is a
test, not a hope.

## The stage

One extra `wlr-layer-shell` surface, created in cat mode only, when the panel's output is
known — at startup with `--output`, otherwise when the compositor's `wl_surface.enter`
tells the panel where it is. It needs its own configure like any layer surface, so until
it has one the panel is drawn alone: the pad and the bubble work, and no part of the
panel's first frame waits for the cat.

- `Layer::Overlay`, the same namespace and app id, the same output as the panel.
- `KeyboardInteractivity::None` and an empty input region: the stage never takes the
  keyboard and never takes a click. The panel keeps both.
- The same buffer scale and the same `wp_viewporter` treatment as the panel, so the two
  surfaces are drawn at one scale and no seam can come from rounding.
- Its own buffer from the panel's `SlotPool`, painted in the same frame pass as the
  panel, damaged whole, committed after it.
- Anchored `TOP | LEFT` and positioned by `top`/`left` margins: with only those two
  anchors the surface keeps its own size and the margins place it anywhere.

### Where it lands

In stage-local coordinates the stage is a square `S`, and the panel's bar card is to its
right. The landing rect puts

- the stage's right edge on the bar card's **left** edge, and
- the stage's vertical centre on the bar card's vertical centre.

So `stage_x + S = card_left` and `stage_y = bar_centre − S/2`. That one identity gives
all three contacts for free:

- the paw's tip is `(S, S/2)` in stage coordinates, which is the bar card's left rim,
  vertically centred: the paw holds the pad;
- the reply card's bottom-left corner is `(S, S/2 − bar_h/2 − SEPARATION)`, on the
  stage's right edge, inside its height: the bubble's tail has somewhere to start;
- the only overlap with the panel's surface is the 26 px shadow column at the stage's
  right edge, which is margin, not card.

`S = min(260 * size, room)` where `room = left_margin + SHADOW` is the distance from the
output's left edge to the panel card's left edge. Below 180 logical pixels — flat, not
scaled, because a cat is a cat whatever the text's zoom — the stage is not created and
the mode keeps the pad and the bubble without a cat: a cat squeezed into 40 px is worse
than no cat.

### Where it comes from

The edge is rolled once per run — the process exits with the panel, so a summon is a
fresh roll — from a seed of the clock's nanoseconds and the pid, with no dependency. The
four peek rects are flush with their output edge and centred on the bar card's axis:

| edge | rect |
| --- | --- |
| bottom | `y = output_h − S`, `x` centred on the bar |
| top | `y = 0`, `x` centred on the bar |
| left | `x = 0`, `y` centred on the bar |
| right | `x = output_w − S`, `y` centred on the bar |

No negative margin is needed: the peek pose is drawn cropped by the window edge that sits
on the output edge, so the cat is cut exactly by the screen.

The hop interpolates the window from the peek rect to the landing rect in `HOP_MS`, with
a sine arc `0.35 * S` above the straight line. The window moves, so the cat moves with
it: the compositor never has to draw a cat outside its own buffer.

## The performance

| elapsed | pose |
| --- | --- |
| 0 – 420 ms | **peek**: the head rises over the edge, ears first, and looks twice |
| 420 – 820 ms | **hop**: the window travels the arc, the cat in a leap |
| 820 – 1120 ms | **arm**: the front paw reaches out and lands on the pad's rim |
| settled | **waiting**: sitting, paw out |

The pad is the input from the first frame, so typing starts before the cat has arrived.
The cat's arrival is theatre around a live field, never a gate on it. The moment the
phase leaves `Idle` — the question is sent — the entrance ends: the stage snaps to its
landing rect and the request's pose takes over from there. A cat in mid-air has no mouth
to hang a bubble from, and the alternative, a second clock for the entrance, is a state
machine for a case that lasts a second.

Then, from the phase:

- `Thinking`, 300 ms in: **lick** — the tongue flicks over the paw that was extended, a
  900 ms cycle, until the first text arrives. The lick belongs to `Thinking` alone: once
the answer streams the cat is answering, not thinking. A request that ends sooner is
  not licked.
- `Answering`, first delta: **speak** — the mouth opens and closes once (200 ms) and the
  bubble grows out of the mouth.
- `Answering`, stream ends: **extend** — the paw lifts and settles again (260 ms). The
  bubble stays.

`animations = "false"` collapses all of it: the stage is at the landing rect, in the
waiting pose with the paw out, in the first frame, and the pad and the bubble are drawn
settled.

## The pad

The bar card, drawn as the magnified paw: the same box, the same height, the same
`PAD`, caret, marquee, scroll and hint. What changes is the card's shape and its ink —
the palm pad with four toe bumps inside the rim at its two ends, the rim lit on the side
the cat is on. Nothing about the input, the IME or the key table changes: the text is
still laid out at `layout_width`, and the caret still follows the keyboard.

The magnification is a drawing fact, not a transform: the pad is the bar at the bar's
size, drawn as a paw would look if the camera were on it.

## The bubble

The reply card, with three changes:

1. Its growth starts at the cat's mouth instead of at the bar: `Look` carries the origin
   point, and the growing card is clipped to a rect interpolating from that point to the
   full box. The origin is `Option`: with no stage there is no mouth, and the bubble
   grows from the bar as the default mode's does.
2. A tail, drawn on the stage, from the reply card's bottom-left corner to the mouth. It
   is inside the stage by the landing identity above, so it needs no third surface and
   no change to the panel's bounds. With no stage there is no tail: a bubble whose cat
   is missing must not point at where it would have been.
3. It stays. In cat mode a request keeps the reply card on screen, showing the last
   answer, until the new answer's first text arrives and replaces it. The transcript
   makes this a one-line rule: a turn with nothing drawn yet shows the previous turn's
   rows.

## The cat

A silhouette, not a cartoon: a near-black body (`#101014`) with a rim light along its
upper-left edges in the panel's ink, two bright eyes, two ear triangles, a tail, and the
front paw. A black cat has no interior detail to draw at this size, and a rim light is
what makes a black shape read on any wallpaper — the panel's own card would otherwise
swallow it.

Every pose is a function of the stage's rect and the pose's progress, drawn with
`render::rect`, per-row spans and the coverage rasteriser the chrome already uses. No
glyphs are involved, so no font is loaded for the cat.

## Reading the mode

`mode` is validated in `parse_config` beside `animations`, with its line number, and read
once at startup. It is file-only: no flag, no environment variable, like `font_size`.

## What is left to the builder

- The exact silhouette, the toe bumps, the tongue, the tail's curve: the numbers above
  fix the geometry, not the drawing. `--preview` grows a `cat.ppm` per pose so the art
  can be looked at without a compositor.
- Whether the stage should be created before or after the panel for stacking. It is
  created after, so the cat composites over the panel's shadow; a compositor that orders
  layer surfaces another way only tints the paw's last 26 px, and the spec says so.
- The sizes of the poses inside the stage, as long as the three contacts above hold.

## Known limits

- Two layer surfaces of one client are stacked by the compositor; the landing identity
  keeps the overlap to the panel's shadow margin, so the worst case is a tinted paw.
- A panel dragged into the left edge leaves no room and loses its cat, tail and mouth
  with it.
- The stage appears one configure after the panel, so on a compositor that places the
  panel itself the first cat frame is a frame or two late.
- The stage is one more buffer and one more paint while the cat moves: 260 × 260 logical
  pixels, at most 27 % of the panel's own frame at size 1.
