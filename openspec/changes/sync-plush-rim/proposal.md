# Proposal

## Why

`plush` changed its chrome: a wider and softer drop shadow, a rim lit at nearly
three times the old strength, and a dark band one stroke inside the rim that makes
the edge read as glass with thickness. `aside` copies its chrome from `plush` (via
`wlmenu`), so it now looks like the older version of both.

## What Changes

- **Shadow**: margin 20 → 26, layers `(9, 120, 2)` and `(20, 95, 6)`.
- **Rim**: the lit ink goes from `#ffffff22` to `#ffffff5a` (alpha 34 → 90), the dark
  underside from 60 to 90, and both profiles take plush's new curves — the lit one
  holds through the top corner arcs and then dies quickly.
- **Inner band**: a dark band one pixel inside the rim, on plush's profile, absent
  at the top edge (where there is no wall below the edge) and whole at the bottom.
- **Card gap**: `SEPARATION` 12 → 16, because the wider shadow now reaches 14 px
  above a card and a tighter gap would smear one card's shadow onto the other.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the frame's shadow and rim numbers, and the 16 px card gap.

## Impact

- `src/render.rs`: the shadow constants, the profiles, the inner band in `body`, and
  `SEPARATION`.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`, `CLAUDE.md` rule 4.
- No new dependency, no new file, nothing on disk.
