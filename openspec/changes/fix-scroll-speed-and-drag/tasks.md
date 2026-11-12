# Tasks

## 1. The wheel's distance is read, not guessed

- [x] 1.1 Add `axis_delta`: `value120` first (120 units a notch), then `discrete`, then a wheel source's axis distance in 15-unit notches, with a finger's axis distance left as pixels — verify: a test asserts all three carriers of one notch agree, that a half-notch of 60 units is half a notch, and that a finger's 3.4 px stays 3.4 px
- [x] 1.2 A notch is five rows — verify: the same test asserts 5 rows one notch, and `NOTCH_ROWS` is the only place the number appears
- [x] 1.3 Carry the fraction: a two-finger slide's tenths add up instead of being rounded away — verify: a test sums a hundred small deltas and finds the whole pixels they were worth, in both directions, with nothing dropped and nothing invented
- [x] 1.4 Hand the axis source through `pointer_frame` so a touchpad is never mistaken for a wheel — verify: a test asserts a finger's axis distance is pixels and a wheel's is notches; that the source survives from one event to the next is the live check in 4.2

## 2. The indicator can be dragged

- [x] 2.1 `panel::scroll_track` and `panel::scroll_column`, used by `draw` for the strip it paints — verify: a panel test finds every pixel of the painted 3 px strip inside the column and inside the track
- [x] 2.2 `Viewport::scroll_to`, the wheel's clamp-and-follow rule for a drag — verify: layout tests place the view at the top (following stops), at the bottom (following resumes), past both ends (clamped), and on a transcript that fits
- [x] 2.3 `thumb_top`, the painter's mapping read backwards — verify: a test drags the thumb to the top of the track and gets offset 0, to the bottom and gets the last offset, to the middle and gets half, and past either end and gets an end
- [x] 2.4 A press in the column takes hold instead of anchoring a selection: the grab offset is the press's place in the thumb, or the thumb's middle when the press landed on the track — verify: a test asserts the column is the card's right padding and refuses the text beside it and the margin beyond it, that a press below the thumb returns the middle as its offset, and that a press on the thumb does not move the view
- [x] 2.5 Carry the drag: motion with the button down moves the view, release ends it, and the indicator stays at full ink while held — verify: a test asserts the held indicator's ink at an age past the hold and the fade together

## 3. Docs

- [x] 3.1 `README.md`, `docs/aside.1`, `docs/live-checklist.md`: five rows a notch, the indicator dragged from the padding column — verify: none of the three still says three lines about the wheel, and each names the drag

## 4. Integration

- [ ] 4.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean. Waiting on a machine with the system faces installed: this sandbox has no monospace face, so four font-metric tests fail before and after the change, and they are the only failures
- [ ] 4.2 Run a real session — verify: a wheel notch moves five rows, a slow two-finger slide creeps without sticking, and the indicator drags from top to bottom and back
