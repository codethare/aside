# Proposal

## Why

`wlmenu` moved on: its input field and its result list are now two separate cards
of one glass chrome, with plush's shadow and hairline numbers, and its bar carries
an accent rail only when it is given a prompt label. `aside` still draws the old
single-panel frame with a rail it has no label for, two different greys inside the
reply area, and a panel pinned to the bottom edge of the screen. And a reply
arrives as a flash of new pixels instead of as something the field produced.

## What Changes

- **Chrome follows the current `wlmenu`**: `SHADOW` 20, the two shadow layers
  `(6, 155, 1)` and `(13, 115, 4)` measured from each card's outline, the
  piecewise hairline profiles, the plush inks (`#ebebf5` at 61 % and `#ffffff` at
  96 %), and two cards separated by 12 px, each with its own outline.
- **The accent rail is gone** from the bar: `wlmenu` draws it only for a `-p`
  prompt label, and `aside` has none.
- **One grey.** The bar card and the replies card share `BG_PROMPT`; a reply is no
  longer a filled block of its own, so `BG_CARD` and the per-block padding go away
  and an 8 px gap is all that separates two replies.
- **The highlight is brighter ink, not a rail**: the highlighted line is drawn in
  `FG_SEL` two pixels in from the rest, so no blue line is left anywhere.
- **The bar sits where the eye already is**: its bottom edge sits 38 % of the
  output height above the screen's bottom edge, so the field lands just below the
  vertical centre and the replies above it stay in the resting field of view.
- **The replies card pops out of the bar.** When the replies card first appears it
  grows upward out of the bar in 160 ms with an ease-out, its shadow, hairline and
  text clipped to the height it has reached. A follow-up does not replay it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the anchor margin becomes a share of the output height; the frame
  takes `wlmenu`'s current numbers; the bar has no rail; bar and replies share one
  body tone; the highlight is ink rather than a rail; the replies card appears with
  a reveal animation.

## Impact

- `src/render.rs`: the shadow becomes a multi-card signed-distance pass, the body
  takes one tone and the profile hairlines, the inks change, `BG_CARD` and
  `rounded_rect` are deleted.
- `src/panel.rs`: two cards, the reveal, the highlight as ink.
- `src/main.rs`: the percentage margin and its height cap, the reveal clock and a
  16 ms wake while it runs.
- `src/transcript.rs`, `src/layout.rs`: `Blocks` loses the card padding.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`, `CLAUDE.md` rule 4.
- No new dependency, no new file, nothing on disk.
