# Proposal

## Why

Three mappings disagreed with the pixels, so a pointer selection started and ended
on the wrong characters.

- `char_at` returned a **byte** offset while `Selection` and the copy path count
  **characters**. On CJK or any multibyte row, clicking after one character named
  three, and the band and the copied slice were wrong.
- `reply_spot` turned the pointer's x into text coordinates without multiplying by
  the buffer scale, and without subtracting the row's own indent or the keyboard
  highlight's two-pixel nudge. At 2× the pointer was a whole card-inset off; on a
  list or code row it was the indent off; on the highlighted line it was 2 px off.
- `over_replies` was given the panel's height (shadow margin excluded) but computed
  as if it were the surface height, so the bottom 52 logical pixels of the replies
  card accepted no selection at all: a press there dragged the panel instead.

A resize or scale change also left a selection anchored to row and character
indices that no longer named the same text, and a copy with nothing selected still
said `copied 1 line`.

## What Changes

- `render::char_at` counts **characters**, with the same advance rules as
  `measure`/`draw_text` (missing glyph box, tab stop).
- `reply_spot` scales x, subtracts the card inset, the row's indent and the
  keyboard's 2 px nudge, and uses the same mark the painter uses for that row.
- `over_replies` takes the panel's own height and adds the shadow margin it sits
  inside; selection and the wheel use the whole replies card.
- A re-wrap or scale change clears a pointer selection, because its row indices no
  longer name the same characters.
- `copy_highlight` reports whether anything reached `wl-copy`; `Alt+C`/`Ctrl+C` now
  show the copied-lines note, and an empty selection no longer claims a copy.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the pointer's character is computed in the same units and
  coordinates the band is drawn in, at any scale and indent; a selection that a
  re-wrap invalidated is cleared; a copy that selected nothing is not reported.

## Impact

- `src/render.rs`: `char_at` and its test.
- `src/main.rs`: `reply_spot`, `reply_text_x`, `over_replies`, selection reset on
  rewrap/scale, copy result and note.
- No dependency, nothing on disk.
