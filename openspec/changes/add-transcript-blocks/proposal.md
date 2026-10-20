# Proposal

## Why

The transcript is painted as one flat row grid: every entry — question, answer,
error, note — is just more rows at the same rhythm and the same body colour, and
the highlight is `wlmenu`'s full-width selection band. Nothing on screen says
"this text is the model's answer"; the panel reads as a `wlmenu` list that
happens to contain sentences. An answer needs its own container.

## What Changes

- The transcript becomes a stack of blocks instead of a grid of rows. Each entry
  occupies a block, and blocks are separated by an 8 px gap instead of sharing
  one row rhythm.
- An answer, an error and a note are painted inside a card: a rounded rectangle
  inset `PAD` (16) from the panel edge, radius 12, its own body tone, 8 px
  padding above and below the text, and the text inset another 8 px. A streaming
  answer's card grows with its tail.
- A question stays what it is now: one bright row, no card, which is what gives
  the card its meaning as "the reply".
- The highlight stops being `wlmenu`'s full-width `BG_SEL` band and becomes a
  2 px `BG_SEL` rail at the left edge of the highlighted line, inside the
  block's own inset. `Up`/`Down`/`PageUp`/`PageDown`/`Alt+C` keep their meanings.
- **BREAKING (internal)**: the viewport becomes pixel-based (`top` and `height`
  in buffer pixels) because block spacing is no longer a multiple of `row_h`.
  The 64-row cap on the transcript area is unchanged, so frame cost stays
  proportional to the visible rows.
- The measured parity with `wlmenu` narrows: **frame and bar** stay exactly
  `wlmenu`'s; the transcript area no longer copies its list rows. `CLAUDE.md`'s
  UI rule is amended to say so.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the answering panel's height is measured in block heights, not
  `rows × row_h`; the frame requirement no longer mandates `wlmenu`'s selection
  band inside the transcript; new requirements state that a reply, a failure and
  a note are drawn in their own card and that a question is not.

## Impact

- `src/transcript.rs`: entries carry their block geometry (`y`, `h`), the
  transcript exposes `total_height()`/`row_y()`, `Row` gains block context, and
  the unused `Row::first` goes away.
- `src/layout.rs`: `Viewport` moves from row indices to pixel offsets.
- `src/panel.rs`: paints blocks and cards; the selection band becomes a rail;
  `height_for_rows`/`visible_rows` are derived from the transcript's height.
- `src/render.rs`: one rounded-rectangle fill helper (built on the existing
  `rounded_span`/`rounded_inset`) and a `BG_CARD` token, added to the existing
  WCAG pair list.
- `src/main.rs`: `size_for` uses the transcript's height instead of
  `rows × bar`.
- Docs: `README.md`'s "What it looks like", `docs/aside.1`, and `CLAUDE.md`'s UI
  rule.
- No new dependency, no new file, no disk state.
