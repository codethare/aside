# Design

## Context

The panel is a top-anchored layer surface whose first row is the bar and whose
transcript hangs below it (`panel::draw`, `size_for`, `main::run`). The transcript
holds a row per entry for every kind, questions included, and the viewport
scrolls over that flat list of lines (see `openspec/changes/add-transcript-blocks`
for the block geometry this builds on).

## Goals / Non-Goals

**Goals:**

- The bar at the panel's bottom edge, the replies above it, and the bar holding
  one position on screen while an answer streams in.
- The questions kept for the request and drawn nowhere.

**Non-Goals:**

- A transcript that scrolls with the bar pinned *inside* a fixed-size panel;
  the panel still grows to its content, capped as before.
- Any change to the provider payload, the key table, or the thinking ball.
- Editing or recalling a sent question.

## Decisions

**1. Anchor the surface to the bottom rather than re-anchoring per frame.** With
`Anchor::BOTTOM` and a bottom margin, the panel's bottom edge is fixed, so the bar
— its bottom row — never moves while answers accumulate above it.
*Alternative rejected:* keeping the top anchor and letting the bar descend by a
row per wrapped line, which is the jitter the redesign exists to remove.
*Alternative rejected:* a top anchor whose margin is recomputed each frame to keep
the bottom edge still — the same geometry with more arithmetic and a configure
round-trip per resize.

**2. The bar is painted last, at `panel.h - bar_h`, and the divider becomes its
top hairline.** Mirroring the existing inset keeps the token values identical;
only the origin moves. The transcript is painted from the panel's top edge and
clipped to the area above the bar, so a card scrolled out of view cannot paint
over the field.

**3. A question is an entry with no lines.** `ask` still pushes it and keeps its
text for `request_turns`, but it is never laid out: `Lines` stays empty, so its
block height is zero, it contributes no rows and no gap, and `rewrap` skips it.
That is the smallest change that removes it from the screen without touching what
the provider sees.
*Alternative rejected:* a second list for the request turns, which would duplicate
the transcript's ordering rules.

**4. The highlight falls on reply lines by construction.** `total_rows`, `rows`
and `row_y` already derive from laid-out lines, so with questions unlaid they
address replies alone and `Alt+C` cannot copy a sent question.

## Risks / Trade-offs

- [The bar's caret rectangle is computed from the panel's top] → `report_cursor`
  must move to the bottom row; an input method places its candidate window by it,
  so a stale value puts the popup in the wrong place.
- [Several transcript tests assert questions appear as rows] → that is the
  behaviour the change removes; each is updated to assert the new arrangement and
  the change is named in the commit, not silently re-asserted.
- [Bottom anchoring changes where the panel appears on every compositor] → the
  visible gap stays 32 px, and the `docs/live-checklist.md` step that looks at
  placement is updated with it.
- [A very tall answer now grows towards the top of the screen] → the existing cap
  (`output_h - 64`) already bounds it, and now bounds it in the direction it grows.
