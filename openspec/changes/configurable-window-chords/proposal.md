# Make the window chords configurable

## Why

An `Alt+hjkl` chord already moves the panel in every mode, copy mode included:
`on_copy_key` checks the chord before it looks at its own table, so reading an
answer is when the panel can be moved out of the way. That part of the request is
already the code's behaviour; it has no test of its own, and the plan below adds one
rather than a feature.

What does not exist is the freedom to choose the chord. `alt` and the four letters
are hard-coded in `key_action` and `window_move`, and the step is a constant
(`WINDOW_STEP = 16`, ten times that with `ctrl`). Anyone whose compositor or input
method already uses `Alt+hjkl` cannot move the panel at all, and anyone who wants a
smaller nudge cannot have one. The configuration file is the place that already
answers "what does this panel do on my machine", so it is where this belongs.

## What changes

Five new keys in the same flat `config.toml`, next to the six that exist:

```toml
window_left  = "alt+h"   # the chord that steps the panel left
window_right = "alt+l"
window_up    = "alt+k"
window_down  = "alt+j"
window_step  = "16"      # logical px per press, 1 to 512
```

- Each direction is a chord: one or more of `alt`, `ctrl`, `shift`, then a key. The
  same chord with `ctrl` added steps ten times as far, which is what `Alt+Ctrl+hjkl`
  does today.
- A chord MUST carry at least one modifier. `window_left = "h"` would take `h` from
  the input line in every mode, so it is refused with the file and the line, like a
  `font_size` that is not a size.
- A chord MUST NOT be one the panel already owns (`alt+c`/`ctrl+c` copy, `ctrl+v`
  paste, `ctrl+t` prefix) or repeat another direction, and `super` is refused with a
  message saying the compositor takes it before the panel sees it.
- `window_step` is a number of logical pixels; a step that is not one is an error
  naming the file and the line.
- Unset keys keep today's behaviour exactly: `alt+hjkl`, 16 px, 160 px with `ctrl`.

## What does not change

- The directions stay `h j k l`-shaped in spirit: four keys, one per direction, one
  meaning each. Copy mode's own `h l j k` (the cursor) are untouched, because the
  chords carry a modifier and the cursor keys do not.
- The panel still clamps to the output, so a step large enough to leave the screen
  stops at the edge rather than falling off it.
- The key table is still the contract: the prefix, copy mode and typing keep every
  other key they have.

## Cost

- ~90 lines: a chord parser and its validation in `provider.rs` (where the other
  config keys and their file+line errors live), a resolved `Chords` value carried to
  the key path, and `key_action`/`window_move` taking it instead of constants.
- Three or four new tests, and the copy-mode chord test that should have existed.
- `README`, `docs/aside.1` (CONFIGURATION), `CLAUDE.md` rule 3, live checklist.

## Open questions the shape depends on

1. **Per-direction keys, or only the modifier?** This plan does per-direction chords
   (`window_left = "alt+h"`). The smaller alternative is one key,
   `window_modifier = "alt"`, with `hjkl` fixed — less to get wrong, less to type,
   and it cannot produce a configuration that fights copy mode.
2. **Does the far step get its own chord?** This plan reuses the near chord plus
   `ctrl`. The alternative is four more keys, which also makes a chord that already
   contains `ctrl` reachable at both distances.
3. **Numbers as strings?** This plan keeps the existing file format (`key = "value"`,
   quoted), so `window_step = "16"`. A bare `window_step = 16` would need the parser
   to learn TOML numbers for every key, not just this one.
