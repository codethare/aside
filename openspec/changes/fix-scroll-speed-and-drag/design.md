# Design

## Context

`State::pointer_frame` receives batches of `PointerEvent`s. `wheel()` turns an
`AxisScroll` into buffer pixels: `discrete` notches × 3 rows, else
`absolute × scale` rounded to whole pixels. `Viewport` holds `top`/`height` in
buffer pixels and a `following` flag, and `scroll_by` is the one clamp-and-follow
rule. `press_on` routes a press to a selection (a reply line) or to the panel drag
(everywhere else). The indicator is drawn by `panel::draw` from
`render::scroll_thumb(track_y, track_h, visible, total, top)` — the same arithmetic
that a drag needs, read backwards. `AxisScroll` carries `value120` (120 units a
notch), `discrete` (deprecated) and `absolute`; `PointerEventKind::Axis` carries the
`AxisSource` (wheel, finger, continuous).

## Goals / Non-Goals

- Make both ways of scrolling the replies move as far as a reader expects, and make
  the indicator a thing the pointer can move.
- Non-Goals: kinetic/fling scrolling after the fingers lift, eased/smooth scrolling
  of a wheel notch, horizontal scrolling, double-click or page-jump on the track,
  and any configuration key: this is the one rate the panel scrolls at.

## Decisions

**1. A wheel's distance comes from `value120`, then `discrete`, then the axis
distance.** `value120` is the field a modern compositor fills in and the code never
read; without it a wheel event falls to `absolute`, where one notch is 15 axis units
— half a row. The axis distance is only read as notches when the source says
`Wheel`, because the same field on a touchpad is already pixels.
*Alternative rejected:* inferring the unit from the magnitude (a 1.0 delta is a
touchpad, 15.0 is a wheel) — the two ranges overlap, and a wheel event from an old
compositor would then jump a page.

**2. A notch is five rows.** `add-wheel-scroll-and-prefix-bar` chose three, from the
terminal standard; that standard measures a 17–20 px text line, and this row is
1.5 em (30 px at the default size), so three of them is 90 px against the standard's
50–60 px — a notch and a half per flick, as the user reported. Kitty, whose rows are
also 1.5 em, ships five.
*Alternative rejected:* keeping three and letting the `value120` fix carry it — it
would leave the wheel at the slow edge for every compositor that already sent
`discrete` (the paths had to agree).

**3. A touchpad's fraction is carried, not rounded.** One event's distance is
`delta + remainder`, the whole pixels move the view and the remainder waits: a
two-finger slide sends tenths of a pixel per event, and `.round()` per event threw
every one of them away.
*Alternative rejected:* storing `viewport.top` as `f32` — the view's offset is
compared against row positions for the highlight and the blocks' gaps, and a
fractional one would have to be truncated at each of those.

**4. The drag is `scroll_thumb` read backwards, not a second mapping.** The thumb's
own top travels the track's free span (`track_h - thumb_h`), which is exactly the
span the painter spreads over `total - visible`. One function, `thumb_top`, is that
inverse, and a drag to the top of the track shows the first line and a drag to the
bottom shows the last, by construction.
*Alternative rejected:* moving `top` by the pointer's delta times a pixels-per-track
factor — it drifts, because the factor changes with the transcript's height, and a
streaming answer grows it mid-drag.

**5. The grab column is the card's right `PAD`.** That is where the 3 px strip is
drawn and where no reply text can be: rows wrap at the same `PAD`, so the column
takes nothing from the text. A press on the track outside the thumb centres the
thumb under the pointer instead of ignoring the press, so the 3 px target never has
to be hit.
*Alternative rejected:* the 3 px strip alone (Fitts: 3 px at the far edge), and a
wider column (it would swallow the ends of long lines from the selection).

**6. The painter and the pointer read one geometry.** `panel::scroll_track` and
`panel::scroll_column` are the track and the column; `draw` uses them for the strip
it paints, and the press path uses them for the strip it grabs, so a change to
`PAD`, `SHADOW` or the gap moves both.
*Alternative rejected:* recomputing the track in `main.rs` — the pointer would grab
a strip drawn somewhere else the first time the chrome moved.

**7. Held while grabbed, and following only at the tail.** The indicator is at full
ink for as long as the button is down — the fade's clock is not the pointer's — and
`Viewport::scroll_to` resumes following at the tail and stops it anywhere else,
which is the wheel's own rule.
