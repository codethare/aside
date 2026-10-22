# Proposal

## Why

Three things the panel cannot do. Its replies stop at an arbitrary `output − 64`
rather than at something the user can see ("as tall as the field is high above
the bottom"), it cannot be moved at all, and reading a long answer means holding
`Up` or paging with keys that fight the input line. A keyboard-driven panel whose
scrollback needs a mouse or a repeated key is the awkward part of using it.

## What Changes

- **The replies' height cap is the bar's own gap.** The replies may grow to the
  same height as the distance between the bar and the screen's bottom edge
  (`output_h × 38 %`, clamped so the panel still fits above it), instead of the
  fixed `output_h − 64`. Dragging the panel changes the gap, so the cap follows it.
- **The panel can be dragged** by holding the left button for a quarter of a second
  and moving. It moves by its layer-shell margins, stays inside the output, and the
  position lives only as long as the process does.
- **A tmux-style prefix mode.** `Ctrl+T` waits for one key: `[` enters **vim
  control mode**, anything else (or `Esc`) leaves it. The prefix is a dispatch
  table so later settings commands can be added without new keys.
- **Vim control mode** takes the keyboard away from the input line:
  `j`/`k` move one line, `n`/`p` one page, `h`/`l` to the first/last line,
  `Ctrl+j`/`Ctrl+k` one page, `Enter` still sends, `Alt+C` still copies, and `Esc`
  leaves the mode without cancelling the stream. The bar says `-- VIM --` while it
  is on, so a swallowed keystroke is never a mystery.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the height cap becomes a ratio of the bar's gap; the panel is
  movable by dragging; the keyboard contract gains the prefix and vim modes.

## Impact

- `src/main.rs`: the cap formula, the `Mode` state, the prefix and vim dispatch,
  a `wl_pointer` and the drag, one more wake source while a press is held.
- `src/panel.rs`: the `-- VIM --` label in the bar (no geometry change).
- `src/layout.rs`: nothing.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`, `CLAUDE.md` (key table).
- No new dependency, no new file, nothing on disk.
