# Design

## Where the editor lives

In the bar. `Mode::Editor` is a fourth mode beside typing, the prefix and copy mode;
the bar card grows from one row to the editor's rows, the replies card above it keeps
its size, and the surface grows upward because the bar is anchored to the bottom
edge. Leaving reverses exactly that.

The alternative — `$EDITOR` on a temp file — is the shell's answer and this panel's
one forbidden thing: it writes the question to a path. It would also hand the
keyboard to another toplevel, which the panel has just spent a change learning to
give up and take back. Neither is worth a file.

## The buffer

```rust
pub struct Buffer {
    lines: Vec<String>,
    row: usize,      // 0-based line
    col: usize,      // CHARACTER index into that line, not bytes
    top: usize,      // first line on screen
    cmd: Option<String>,
    pending: Option<char>,   // `g` or `d`, waiting for its second key
    mode: Mode,              // Insert | Normal
}
```

Columns are characters, because everything else in the panel counts characters when
it points at text — the selection, the copy, the copy-mode cursor. Bytes appear only
at the moment a `String` is indexed, through one helper.

`from_text`/`text` convert between the buffer and the input line's `String`, joined
with `\n`. The input line already carries newlines: a paste can contain them and
`send()` hands the whole string to the provider. So `:wq` is a copy, not a
transformation, and a question written over four lines reaches the model as four
lines.

The buffer is capped at `CAP = 16 * 1024` bytes: a question, not an answer. Past the
cap a key that would grow the text does nothing and the label says so, rather than
silently dropping characters.

## Modes inside the mode

`Mode::Editor` has its own two: `Insert` and `Normal`. It starts in **insert**,
because a key that opens a window must not swallow the letters typed after it — the
same reason the prefix mode shows its hint before the key lands. `Esc` gives normal;
`Esc` again leaves the editor and keeps the text; `:q!` leaves and drops it.

The label on the bar's right says which one is on — `-- INSERT --`, `-- NORMAL --` —
or, on the command line, the answer to a command that did not parse: `E: not a
command`.

## Keys

Two tables, and nothing shared between them by accident:

```rust
fn normal_key(keysym: Keysym) -> Option<NormalKey>
```

`NormalKey` is `Left`, `Right`, `Up`, `Down`, `LineStart`, `LineEnd`, `WordForward`,
`WordBack`, `DeleteChar`, `DeleteLine`, `InsertAt`, `AppendAfter`, `AppendEnd`,
`InsertStart`, `OpenBelow`, `OpenAbove`, `First`, `Last`, `Command`, `Leave`.

`h l j k 0 $ w b x dd i a A I o O gg G : Esc` in that order. `dd` and `gg` are two
keys: the first sets `pending`, the second acts, and any other key clears it — which
is what makes a half-typed `d` harmless.

In insert the existing typing table applies unchanged, with one difference and one
addition: `Enter` is a newline instead of a send, and `Esc` changes mode. That is a
three-line branch in the key routing, not a second table.

The command line takes printable keys, `Backspace`, `Esc` (cancel, back to normal)
and `Enter` (run):

| typed | verdict |
|---|---|
| `:wq`, `:q`, `:x` | `Write` — the text goes back into the input line, the bar returns to one row |
| `:q!` | `Discard` — the input line keeps whatever it held before `Ctrl+G` |
| anything else | `Unknown` — the bar keeps the command line and says `E: not a command` |

`:q` writes rather than refusing, unlike vim: there is no separate file that could be
unsaved, and the buffer the user was editing is the thing they wanted in the input
line. `:q!` is the door out without it.

`Ctrl+G` in either mode leaves and keeps the text — the key that opened the window
closes it, which is the only meaning a key can have and stay learnable.

`Alt+hjkl` and `Alt+Ctrl+hjkl` keep moving the panel in the editor too: the chrome's
chords are not a mode's to take.

## Rows on screen

`VISIBLE = 8`. `Buffer::scroll` keeps the caret's line inside `top..top + VISIBLE`;
the panel asks the buffer for `top`, so what is painted and what the caret does are
the same decision made once.

Long lines scroll sideways in the first version, reusing the horizontal scroll the
bar already has for its one row (`panel::caret_scroll`). Wrapping inside the editor
is a second piece of geometry — the caret would have to know which wrapped segment
it is on — and a line that scrolls is still editable, which is all a multi-line
question needs. It is written down as a limit rather than discovered later.

The command line, while it is being typed, is one more row under the buffer's rows.
So `Buffer::rows()` is `min(lines, VISIBLE) + cmd.is_some()`, and the bar's height is
`row_h * Buffer::rows()`.

## The surface

`size_for(phase, output_h)` becomes `size_for(phase, output_h, editor_rows)`, where
`editor_rows` is 1 outside the editor. The bar's height is `row_h * editor_rows`, and
the surface is `bar + SEPARATION + room_for_replies` when the replies card is up.

Entering and leaving are one resize each, exactly like a phase change: the bar card
is the bottom card, so the surface grows by the editor's extra rows and the replies
card, which sits above it, is unmoved. No animation: the shape machinery animates
between `Phase`s, and the editor is not one; a bar that grows by seven rows in one
frame reads as a window opening, which is what it is.

## A short output

The editor's rows count against the output's height, not against infinity: the
surface is `bar rows + SEPARATION + room_for_replies`, and a small output with the
replies card up leaves less than eight rows. The rows are clamped to what fits above
the anchored bottom margin, and one row always fits, so the editor opens on a small
screen with one line visible and still works; the replies card keeps its own cap,
which is what gives way first.

## The input method

Insert enables the text input, normal disables it. Composing while `h` and `j` are
motions would eat the motions — the same reason copy mode disables it. `report_cursor`
reports the caret's rectangle on the caret's own editor row, not the bar's single row,
so a candidate window lands where the text is.

## What it does not do

- No visual selection, no `y`/`p`, no undo, no `:s` — a first version that covers
  typing, correcting and leaving.
- No file, no register, no external editor.
- No soft wrap (see above).

## Cost

`src/editor.rs` is about 350 lines with its tests: a `Vec<String>` with a character
caret is not a text editor, and the key tables are the only part that grows. The
painter gains a loop over visible rows where it had one row; the frame cost stays per
visible row, and the editor draws at most eight of them.
