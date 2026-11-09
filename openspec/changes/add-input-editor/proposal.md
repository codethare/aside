# A temporary editor in the input bar

## Why

`README` says it plainly: "There is no newline key: the input is one logical line
that soft-wraps, and a multi-line question can be pasted." A pasted question is the
only way to send a prompt with structure — a list of requirements, a diff, a few
lines of code — and once pasted it can only be corrected through a one-row bar that
scrolls sideways.

`Ctrl+G` gives the input a real editor: the bar grows into a small modal window over
the same surface, vim's keys move a caret inside it, and `:wq` puts the text back in
the input line and returns the bar to one row.

## The one rule this must not break

**Nothing is written anywhere but memory** — and `vim` on a temp file is exactly a
file. So the editor is *inside* the panel: on `Ctrl+G` the bar itself becomes the
window; no process is spawned, no path is touched, and the buffer lives in the same
`String` the input line does. This is the same constraint that keeps the transcript
in RAM, applied to the one place a shell would reach for `$EDITOR` and a
`mktemp`.

(If a real `vim` on a temp file is what you want instead, that needs rule 1 lifted
explicitly: it would write the question to a file, spawn a terminal, lose the
keyboard to a toplevel and read the file back. The plan below does none of that.)

## What it looks like

```text
┌──────────────────────────────────────┐
│ why is the socket not ready          │   ← the bar, one row, as usual
└──────────────────────────────────────┘
          Ctrl+G
┌──────────────────────────────────────┐
│ ## what I need                       │
│                                      │   ← up to 8 lines, the caret on one
│ - the socket keeps returning EAGAIN  │
│ - poll, then retry                   │
│ :wq                                  │   ← the command line, only while typing one
└──────────────────────────────────────┘
          :wq  (or Esc Esc)
┌──────────────────────────────────────┐
│ ## what I need - the socket keeps ...│   ← back to one row, the text kept
└──────────────────────────────────────┘
```

The replies card above keeps its size; the panel grows taller by the editor's rows
and shrinks back on the way out. The bar stays anchored to the bottom edge, so the
extra rows take room above it, not below.

## Keys

Starting state is **insert**, because a key that opens a window must not swallow the
letters typed next — the same reason the prefix mode shows a hint. `Esc` gives
normal, `Esc` again leaves.

| Mode    | Keys |
| ------- | ---- |
| insert | every typing key, `Enter` for a newline, `Backspace`/`Delete`, `Left`/`Right`/`Up`/`Down`, `Home`/`End`, `Ctrl+Left`/`Ctrl+Right` for words, `Esc` for normal, `Ctrl+G` to leave keeping the text |
| normal | `h l` a character, `j k` a line, `0 $` the ends of the line, `w b` a word, `x` the character under the caret, `dd` the line, `i a A I` into insert, `o O` a new line below or above, `gg G` the first or last line, `:` the command line, `Esc` to leave keeping the text |
| command | `:wq`, `:q`, `:x` write back and leave; `:q!` leave and drop what was typed; `Backspace` edits; `Esc` returns to normal; anything else answers `E: not a command` and stays |

`Alt+hjkl` / `Alt+Ctrl+hjkl` keep their meaning — they move the *panel* — in the
editor too, like every other mode.

## What it costs

- A new module, `src/editor.rs`: a `Buffer` of lines with a character caret, the two
  key tables, the command parser. Roughly 350 lines with its tests.
- The panel grows a state: the bar is not always one row. `size_for` learns the
  editor's row count, and the surface is resized on entering and leaving, once each,
  the way a shape change already does.
- `panel::draw_bar` learns to paint several rows and a caret on one of them; the
  horizontal scroll it already has for the one row is reused per line.
- Long lines scroll sideways rather than wrapping, in the first version: wrapping
  inside the editor is a second piece of geometry (a caret that lands on a wrapped
  segment) and is not needed to write a multi-line question.
- The text is capped at 16 KiB, well under what a question needs and far under
  `REPLY_CAP`.

## Impact

- `src/editor.rs` (new), `src/main.rs` (`Mode::Editor`, `Ctrl+G`, the IME handshake,
  `report_cursor`, `size_for`), `src/panel.rs` (the multi-row bar), `src/layout.rs`
  (`Input` gains nothing; the editor converts to and from its text).
- `specs/chat-panel`: the keyboard contract gains the editor, and a requirement says
  what the editor is and what it does with the text.
- `README`, `docs/aside.1`, `CLAUDE.md` rule 3 and rule 1 (the editor is the proof
  that nothing needs a file), `docs/live-checklist.md`.
- No dependency, and no process.
