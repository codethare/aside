# Proposal

## Why

Both ways of scrolling the replies are too slow, with a wheel and with a touchpad.

The wheel is slow for a reason the code hides: `wheel()` reads only
`wl_pointer.axis_discrete`, which the protocol deprecates, and it never reads
`axis_value120` — the field a modern compositor fills in for a notched wheel (120
units is one notch). A wheel event that carries no `discrete` falls through to the
`absolute` branch, where one notch is 15 axis units, so a notch moves 15 px: half of
the 30 px row at the default size, instead of the three rows the spec promised. The
touchpad loses distance the other way: each event's delta is rounded to whole pixels
before it is added, so the sub-pixel events a slow two-finger slide sends are
dropped one by one.

The 3 px scroll indicator is also decoration: nothing in the pointer path can touch
it, so the only way to reach a line is the wheel or the keys.

## What Changes

- **A wheel notch moves five lines, and its distance is read from the field that
  carries it.** A notch is taken from `value120` first, then from `discrete`, then
  from the axis distance of a wheel source — never as one axis unit. Five rows is
  the terminal standard's rate for a row this tall: this row is 1.5 em, so three of
  them is 90 px where the standard's three lines is some 50 px.
- **A touchpad's delta is carried, not rounded away.** The fractional remainder
  survives into the next event, so a slow slide creeps instead of sticking.
- **The indicator can be dragged.** A press in the replies card's right `PAD`-wide
  padding column takes hold of the thumb: motion places the view where the thumb
  is, a press on the track outside the thumb centres it under the pointer first, and
  the thumb stays at full ink for as long as the button is down.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the wheel's distance is read from `value120`/`discrete`/the axis
  distance, a notch is five lines, no part of a scroll is dropped, and the scroll
  indicator is draggable from the card's right padding.

## Impact

- `src/main.rs`: the axis handling, a fractional scroll remainder, the indicator's
  track and the press/motion path that grabs it.
- `src/layout.rs`: `Viewport::scroll_to`, so a drag is one clamp-and-follow rule
  with the wheel's.
- `src/panel.rs`: nothing — the track the painter draws is already the track the
  pointer grabs, and it is derived from the same numbers.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`.
- No new dependency, no new file at runtime, nothing on disk.
