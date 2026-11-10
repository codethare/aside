# Design

## The question is not the input line

The obvious implementation is to stop clearing the input on `Return` and let the bar
draw it. It fails on the follow-up: the panel is typable while an answer streams, so
the first letter typed would land in front of the question just sent, and the reader
would be editing the question instead of asking a new one.

So the question is its own value — the text and the instant it went out — and the bar
shows it *instead of* the input line. The input line is empty. Nothing typed can reach
the question, because nothing is pointed at it.

## Why the age, not an offset

The bar scrolls by a pixel offset, and the offset is a pure function of how long the
question has been out. Two reasons:

- One frame is one number. No per-frame accumulation, no remembering where the scroll
  got to, and a dropped frame changes nothing — a drag of the window or a resize tows
  the offset along with it without a catch-up.
- It is testable without a clock. `marquee(elapsed, travel, s)` takes the elapsed time
  as an argument and returns a distance: the test asserts the ends, the hold and the
  turn, with no sleeping.

`travel` is measured where the geometry already is — inside the bar, as the text's
advance minus the room the bar gives it — so the width math lives in one place.

## The speed and the hold

`MARQUEE_SPEED = 26` px a second at scale 1, and `MARQUEE_HOLD = 700 ms` at each end.

The bar is read, not skimmed: a question is one line of prose, and a fast crawl is a
flicker that makes the reader chase the words. 26 px/s moves one character about every
half second at the design size, which is reading pace. The hold exists so the turn
rounds visibly — without it the direction change is a stutter that looks like a bug.

Both scale with the panel's scale, like every other pixel length.

## What stays put

The caret is not drawn while a question is out: there is no caret, because there is no
editable line. The pre-edit is not drawn either, and no key but `Esc` is honoured while
the request has not answered — both were already true.

The label (`-- COPY --`, `copied N lines`) still owns the bar when copy mode does: the
question is shown only in typing mode, so one meaning per key survives. Entering copy
mode while a question is out therefore swaps the bar to the copy label and back.

## Rulings

- **The question leaves on any key that reaches the line, not only on a letter.** A
  caret move on an empty line changes nothing on screen, so keeping the question
  through it would leave a bar that shows text nobody can type into. Cost if wrong: a
  reader who presses `Left` to look at the line loses the question early. The text is
  still in the transcript's request, so the request itself is unaffected.
- **The question survives the first token.** `Phase::Answering` starts when text
  arrives, and the question outlives that phase change: the reader asked it, and the
  request is not over. It clears when the stream ends, not when it starts.
- **A refusal clears it immediately.** If the request cannot start at all, there is
  nothing in flight to show.
