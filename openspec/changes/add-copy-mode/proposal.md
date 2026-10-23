# Proposal

## Why

Vim mode tried to be two things at once: it moved the panel, and it walked the
replies. tmux already has the shape for the second — a copy mode you enter, move a
cursor in, select with, and leave by copying — and the first is one modifier
chord away, available in every mode without a mode to enter.

## What Changes

- **Vim mode is removed.** The panel moves with `Alt+hjkl` (16 px) and
  `Alt+Ctrl+hjkl` (160 px) from any mode, with no mode to enter or leave.
- **A tmux-style copy mode replaces it**: `Ctrl+T` then `[` enters it, a visible
  cursor moves by character, line, word, half page and page, `Space` starts a
  selection, `Enter`/`y` copies it and leaves, `q` leaves, `Esc` clears a selection
  before it leaves. The bar reads `-- COPY --`, and after a copy it says
  `copied N lines` for a moment.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: vim mode's requirements are removed; the keyboard contract loses
  its vim column and gains the window chords and copy mode; copy mode's keys,
  cursor, selection, copy-and-leave and feedback are specified.

## Impact

- `src/main.rs`: the mode set, the copy-mode key table and cursor, the window chords.
- `src/panel.rs`: the copy cursor is drawn; `-- COPY --` and the copy note in the bar.
- `src/layout.rs`, `src/transcript.rs`: word motion over the reply lines.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`, `CLAUDE.md` rule 3.
- No new dependency, no new file, nothing on disk.
