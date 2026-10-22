# Design

## Context

`main::size_for` caps the answering panel at `output_h − edge_gap − 32`, with
`edge_gap` a fixed 38 % of the output. There is no pointer at all:
`SeatHandler::new_capability` returns early for anything that is not a keyboard.
`key_action` is a pure table that gives `h/j/k/l` to `Key::Text`, and `Esc` to
cancel-or-close. The panel is placed by its layer-shell anchor and margins; there
is no other way to move a layer surface.

## Goals / Non-Goals

- A cap the user can see the reason for, a drag that moves nothing else, and a
  navigation mode that cannot type.

**Non-Goals:**

- Persisting the panel's position (privacy rule: nothing on disk) or animating the
  drag.
- A general vim implementation: no counts, no registers, no `/` search, no `gg`.
- Reusing the vim mode for editing the input line; it moves the highlight only.
- Any new configuration key or flag.

## Decisions

**1. The cap is one ratio, not a second constant.** `reply_cap(gap, output_h)` is
`gap` clamped to the room left above the bar (`output_h − gap − SHADOW`). One
number (`REPLY_SHARE = 1`) says how tall the replies may be relative to the field's
own height above the bottom; the clamp is what makes a dragged-to-the-top panel
still fit. The drag offset feeds `edge_gap`, so the cap follows the drag with no
extra state.
*Alternative rejected:* keeping `FAR_ROOM` and adding a share — two answers to the
same question, and the one the user asked for would be the second.

**2. The drag is margins plus an offset, and it starts after a hold.** A layer
surface moves only by `set_margin`; the offset is `(left, bottom)` from the
anchored place, so the default position stays the default and the drag never has to
know the anchor. `DRAG_HOLD_MS = 250` before motion counts, so a click on the panel
is not a one-pixel jump; the press is a deadline checked by the loop rather than a
timer of its own, and the loop wakes every 30 ms while a press is pending.
*Alternative rejected:* a compositor-side move (there is no protocol for moving a
layer surface) or an xdg-shell window (the whole point is that the compositor does
not place it).

**3. Two modes and one dispatch table each.** `Mode = Typing | Prefix | Vim`. The
prefix returns to typing after exactly one key — `[` enters vim mode and everything
else does nothing — and lives in `prefix_action(keysym) -> Prefix`, the site later
settings keys go. Vim keys live in `vim_move(keysym, ctrl, page) -> Option<i64>`,
returning a row delta so the existing `move_highlight` does the work and no second
scrolling path is written.
*Alternative rejected:* one flat key table with a mode flag in every arm, which is
the table `key_action` already is, times three.

**4. The mode is visible in the bar's own line.** `-- VIM --` is drawn
right-aligned in the bar in the secondary ink, and the caret is hidden. No new
geometry, no new colour token, and the panel says why `j` did not type.

**5. The input method is disabled, not merely ignored.** Entering vim mode
disables the text input and drops any pre-edit; commits arriving while the mode is
on are ignored as a second line of defence. Otherwise fcitx keeps composing
`j`/`k` and the mode is a lie for anyone typing Chinese.

## Risks / Trade-offs

- [A drag re-commits the surface on every motion event, which some compositors
  answer with a configure round-trip] → the margins change and the size usually
  does not, so the damage is a commit; if it stutters, the fix is to apply the
  margin on the frame rather than per motion event, not to move by pixels.
- [The cap depends on the drag, so dragging while an answer streams can resize the
  panel under the reader] → the drag is a deliberate act and the reply area
  re-clamps; the transcript is not re-wrapped, because the width does not change.
- [`h`/`l` have no horizontal axis in one column, so their meaning is a choice] →
  first/last line, the same "far" reading vim gives them in a list; one constant
  away from any other pair.
- [Vim mode swallows keys, including letters the user meant to type] → the bar
  label and the missing caret are the signal, and `Esc` always leaves.
- [A press held while the panel is dragged outside the output] → both margins are
  clamped to the output on every motion event.
- [The prefix steals `Ctrl+T`] → it was free: `key_action` ignored it before, and
  it is not a text key.

## Migration Plan

Nothing on disk, nothing to migrate. The key table in `README.md`, the man page,
`docs/live-checklist.md` and `CLAUDE.md` are updated in the same change.
