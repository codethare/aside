# Design

## Context

`Mode` is `Typing | Prefix | Vim`; vim mode owns `hjkl` for the panel and
`n`/`p`/`Ctrl+n`/`Ctrl+p`/`g`/`G` for the highlight. `Selection` is already
character-precise and shared with the pointer, `panel::Selection::columns` cuts a
row to the characters it covers, and `render::char_at`/`advance` convert between a
pointer's x and a character.

## Goals / Non-Goals

- A cursor that can be moved in the replies and a selection that copies, with tmux's
  keys; the panel moved by a chord instead of by a mode.

**Non-Goals:**

- `/` and `?` search, `f`/`t` character jumps, numeric prefixes, rectangular or
  block selection, a scrollback of earlier turns (there is only the current one).
- Copy-on-select, or a mouse selection that also enters copy mode.

## Decisions

**1. `Mode::Copy` owns a cursor, and the selection is the one already there.**
`copy_cursor: (usize, usize)` is the row and character; `selection` is the same
`panel::Selection` the pointer writes, so `Alt+C`, `Ctrl+C` and copy mode's `Enter`
all copy the same thing and a pointer drag inside copy mode still works.
*Alternative rejected:* a second selection type for copy mode — two sources of truth
for "what is selected".

**2. Word motion lives next to the text, not in the key table.** `Transcript::word_forward`
and `word_back` take a position and walk rows, breaking on whitespace and on the CJK
boundaries `layout::breaks_after` already defines, so `w`/`b` agree with the wrapper
about what a word is.
*Alternative rejected:* splitting on `' '` only — `w` would walk a Chinese sentence
as one word.

**3. The window chords are checked before the key table, in every mode.**
`Alt+hjkl` is one branch at the top of `on_key`; copy mode, the prefix and typing all
see it, so the panel can be moved while reading.
*Alternative rejected:* a `prefix` command for moving — two keys where one chord
does it, and unusable while copy mode has the keyboard.

**4. The copy note is a deadline, like the reveal was.** `copied_at` holds the instant
and the count; the bar's label is `copied N lines` while it is under 1.2 s old, and
the loop wakes every 100 ms while it is on screen. No new state machine.
*Alternative rejected:* a transcript entry for the copy — every copy would grow the
box, and the box is the answer.

## Risks / Trade-offs

- [A cursor drawn as a 1 px bar is easy to lose on a busy line] → it is drawn at full
  height of the row in the primary ink, and the highlighted line keeps its own mark,
  so there are two things to see.
- [`Alt+hjkl` may collide with a compositor or IME chord] → it is free in this panel
  today, and the chords are one table entry to change.
- [Removing vim mode removes `n`/`p` for the highlight in typing mode] → typing mode
  keeps `Up`/`Down`/`PageUp`/`PageDown`, and copy mode is the thorough way to read.
