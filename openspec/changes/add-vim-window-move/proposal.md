# Proposal

## Why

Vim mode can move the highlight but not the panel. The panel's position is a
thing the user sets by hand, and with vim mode owning the keyboard the pointer is
the only way to change it — the mode is meant to spare exactly that.

## What Changes

- In vim mode `h`, `j`, `k`, `l` move the **panel** on the desktop: 16 px a press
  left, down, up, right, and `Ctrl+hjkl` ten times that. The panel is clamped to
  the output on all four sides, and the height cap follows the new gap.
- The transcript keeps its own keys: `n`/`p` a page, `Ctrl+n`/`Ctrl+p` a line,
  `g`/`G` the first and last line — the line and ends keys `j`/`k` and `h`/`l`
  used to be, moved so the spatial pair can move the window.
- The drag and these keys go through one clamped `move_panel`, so a panel cannot
  be pushed off the output by either.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: vim mode's `hjkl` moves the window, the transcript keys move to
  `n`/`p`/`Ctrl+n`/`Ctrl+p`/`g`/`G`, and the panel's position is clamped on all
  four sides however it moves.

## Impact

- `src/main.rs`: a clamped `move_panel`, `window_move`, the re-arranged vim table,
  and a vertical clamp on the drag.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`.
- No new dependency, no new file, nothing on disk.
