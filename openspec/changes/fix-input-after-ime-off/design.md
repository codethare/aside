# Design

## Context

`Ime::wants_raw_keys()` is `!live`, and `live` is set by the first pre-edit,
commit, deletion or `done` the input method delivers. `on_key` inserts keysym text
only while `wants_raw_keys()`. There is no protocol event that says "the input
method is no longer there": switching fcitx off or quitting it produces silence,
which is indistinguishable from "nothing has been typed yet".

## Decisions

**1. Arbitrate in the event round, not in time.** A text key pressed while `live`
is not inserted; it is appended to a held buffer. `tick` runs after every dispatch
and flushes that buffer as typed text when the round ended without the input method
claiming it. Any input-method event in the round clears the buffer, because the
character the reader typed is part of its composition.
*Alternative rejected:* a timeout ("if the IME says nothing for N ms, type it") —
a timer per keystroke, and on a slow IM the text would arrive out of order.
*Alternative rejected:* an off switch for the input method — the compositor and the
IM own that state, not the panel.

**2. The rule lives in `Ime`, not in `main`.** `hold(text)`, `claimed()` and
`flush()` are a pure state machine, so the four outcomes (no IM, IM claims with a
pre-edit, IM claims with a commit, IM silent) are tested without a compositor.

## Risks / Trade-offs

- [An input method that answers in the *next* event round would double-type one
  character] → text-input-v3 answers a key in the same round as the key event
  (`preedit` then `done`, or `commit` then `done`); the held buffer is flushed only
  after the round that contained the key.
- [A held key is inserted a few microseconds late] → it is the same frame: the
  flush happens before the frame is drawn.
