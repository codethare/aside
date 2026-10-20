# Design

## Context

Today the whole geometry chain assumes one uniform row grid (see proposal.md -
Why):

- `Transcript::rows()` flattens every entry into a single `Vec<Row>`; `Row::first`
  was meant to mark a turn's first row but the painter never reads it.
- `Viewport` holds `top` and `height` as row indices, and the painter places row
  `i` at `panel.y + transcript_top + i * row_h`.
- `size_for` in `main.rs` derives the panel height as `bar + 1 + rows * row_h`.

Two standing constraints shape any change here: frame cost is per visible row,
never per transcript row (`CLAUDE.md` rule 5), and wrapping happens once per
resize, once per delta — a pixel-heavy layout recomputed per frame is the bug the
current code already avoids.

## Goals / Non-Goals

**Goals:**

- Blocks with their own geometry, so a card can have padding that is not a
  multiple of `row_h`, without giving up the flat frame cost or the 64-row cap.
- Card painting, rail highlight and the streaming growth path as specified in
  `specs/chat-panel/spec.md`.

**Non-Goals:**

- Markdown, code blocks, tables, or a different tone per entry kind — a failure
  and a note get the same card as an answer, as they share its row treatment today.
- Per-kind wrap widths, scrollbar redesign, tweened growth, or fractional scale.
- Changing which keys do what.

## Decisions

**1. Each `Entry` caches its block geometry (`y`, `h`), maintained incrementally.**
`push` sets `y = previous end + gap`; `delta`/`finish`/`cancel`/`fail` recompute
only the tail entry's `h` (and its `y`, which the growing `h` cannot change);
`rewrap` recomputes the chain once per resize. `total_height()` and `row_y()`
are then O(1).
*Alternative rejected:* walking the entries per frame to accumulate heights —
O(entries) per frame, which grows with the transcript and breaks the frame-cost
rule. *Alternative rejected:* parallel prefix-sum vectors — same numbers, more
state to keep in sync for no gain, since only the tail ever grows.

**2. The viewport is measured in pixels, not rows.** `top` and `height` become
buffer pixels; `ensure_visible` takes the line's `y` and `row_h`; `Up`/`Down`
step by `row_h` so "one line" keeps meaning one text row; the 64-row cap stays a
pixel cap of `MAX_ROWS * row_h`, so card padding cannot make the panel grow past
it.
*Alternative rejected:* spacer rows for gaps and padding. A 30 px gap is too much
air, and `Alt+C` would be able to copy an empty spacer.

**3. One wrap width for every kind:** `panel_width - 2 * PAD - 2 * CARD_PAD`,
i.e. the card's inner width. A question therefore wraps narrower than the panel
could — harmless, and it keeps a single `max_w` in the transcript, so a resize
still re-wraps once.
*Alternative rejected:* per-kind widths, which would double the state a resize
must refresh and let a question's rows disagree with the card's grid.

**4. The card is a rounded fill built on what `render` already has.**
`rounded_span`/`rounded_inset` give the outline row by row, exactly as the panel
body does, so no new geometry is written. The card tone is a new `BG_CARD` token
between `BG_NORMAL` and `BG_PROMPT` (`#262626` at `0xd8`), added to the existing
WCAG pair test together with `FG_NORMAL`.

**5. The highlight is a 2 px `BG_SEL` rail at the block's inset.** No band: the
band is the list signature the change exists to remove. The rail is painted from
the line's `y` and `row_h`, so it cannot move text or resize anything.
*Alternative rejected:* a low-alpha band inside the card — the same list look,
only dimmer. *Alternative rejected:* dropping the highlight — `Alt+C` would lose
its subject and the key table in `specs/chat-panel` would change.

**6. An empty entry occupies no height and draws nothing,** so a freshly opened
answer creates no gap between a question and the card that has not started. The
gap is drawn only before a block whose height is not zero.

## Risks / Trade-offs

- [Scrolling regressions, the one real risk] → the viewport's existing tests move
  to pixel semantics and gain the case that matters: a reader scrolled up does
  not move when the tail card grows.
- [`the_selection_is_clipped_inside_the_border` no longer describes the product]
  → it is replaced by the rail test, and the behaviour change is named in the
  commit message rather than quietly re-asserted.
- [The `wlmenu` parity claim in README/`CLAUDE.md`/man page becomes false] →
  rule 4 of `CLAUDE.md` and the frame sentence in all three are narrowed to
  "frame and bar" in the same change.
- [Card edges cost more than a plain rect] → only the visible cards are filled,
  at most a few dozen outline rows per frame; the shadow, the body and the text
  still dominate.
- [A long single-line answer in a narrow card] → wrapping uses the card's inner
  width, so the text cannot overflow the card; the existing CJK/Latin wrap tests
  cover the width change.

## Migration Plan

No state exists on disk, so there is nothing to migrate. Archiving is ordered:
`add-aside-chat-panel` and `add-panel-states` must be archived before this change,
because this delta modifies requirements that only exist in those changes'
specs — `openspec validate --strict` already reports that archive would refuse it
otherwise.
