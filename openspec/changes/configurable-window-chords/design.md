# Design

## One chord, resolved once, carried to the key path

`key_action(keysym, alt, ctrl, has_text)` is a pure function over the fixed table, and
`window_move(keysym, ctrl)` turns a letter into a delta. Both are called from the
event loop, which already holds the seat's modifiers.

```rust
/// The four directions' chords and the two step sizes, resolved from the config once
/// at startup: the key path must not parse a string per keystroke.
pub struct Chords {
    left: Chord,
    right: Chord,
    up: Chord,
    down: Chord,
    step: i32,
    far: i32,
}

struct Chord {
    alt: bool,
    ctrl: bool,
    shift: bool,
    keysym: Keysym,
}
```

- `Chords::default()` is today's behaviour: `alt+h`, `alt+l`, `alt+k`, `alt+j`,
  16, 160.
- `Chords::from_config(&[(String, String)]) -> Result<Chords, ConfigError>` reads the
  five keys, reusing the `ConfigError { path, line, message }` shape the other keys
  use, so a bad chord names its line exactly as a bad `font_size` does.
- `key_action` takes `&Chords` and returns `Key::Window { dx, dy }`: the delta is
  decided where the chord is matched, so the handler moves and nothing else.
  `window_move` disappears into `Chords::delta`.

Shift is a new modifier on the path. `self.modifiers` is already the seat's
`Modifiers`, so `key_action` gains a `shift: bool` parameter and nothing new is read
from the protocol.

## Parsing a chord

`"alt+ctrl+h"` → one or more modifiers then a key, case-insensitive, whitespace
allowed around the parts. The key half is a small table, not a keysym database:

| written | keysym |
| --- | --- |
| `a`..`z`, `0`..`9` | the character |
| `left`, `right`, `up`, `down` | the arrows |
| `pageup`, `pagedown`, `home`, `end` | the same keys the table already names |

Anything else is refused: the panel can only observe what it already maps, and a
chord for a key it cannot see would be a silent no-op.

Refused, each with its own message and the file and line:

- **no modifier** — it would take a plain key from the input line;
- **`super`** — the compositor's bindings run first, so the panel never sees it;
- **a key the panel already owns** — `alt+c`, `ctrl+c`, `ctrl+v`, `ctrl+t`, and a
  direction that repeats another;
- **more than one key in the chord** (`alt+h+l`) — one press, one key.

## Steps

`window_step` is a number of logical pixels in `1..=512`, like `font_size`'s bounded
number: outside it, the file and line are named and nothing opens. `far` is
`step * 10` unless `ctrl` is already in the chord — then the far chord would be the
near one and only the near step is reachable, which the documentation says.

Deltas are logical px, so the buffer scale multiplies once inside `move_panel`, as it
does today: the panel moves the same distance on a 1× and a 2× output.

## Where the modes meet

```text
on_key(event)
  match key_action(&self.chords, keysym, alt, ctrl, shift, has_text)
    Key::Window { dx, dy } => self.move_panel(dx, dy)   // every mode

on_copy_key(event)
  if let Some((dx, dy)) = self.chords.delta(modifiers, keysym) { move; return }  // first
  ... the copy table ...
```

Copy mode keeps the chord before its own table, as it does today: `Alt+h` is a window
move there, `h` alone is the cursor. Nothing else in copy mode changes.

## Tests

- `a_chord_is_a_modifier_and_a_key` — parse `alt+h`; each direction; case and spaces.
- `a_chord_without_a_modifier_is_refused` / `a_chord_the_panel_owns_is_refused` /
  `a_chord_with_two_keys_is_refused` — each error, and the line it names.
- `the_step_is_bounded` — `"0"` and `"513"` refused, `"24"` accepted.
- `the_defaults_are_todays_behaviour` — an empty config gives `alt+hjkl`, 16, 160.
- `a_configured_chord_moves_the_panel` — `ctrl+shift+h` with `window_step = "24"`
  gives `(-24, 0)`; the same with `ctrl` added is the far step.
- `copy_mode_takes_the_chord_before_its_own_table` — the pure delta lookup on the
  copy-mode path, which is the behaviour that already exists and has no test.

## See also

The proposal's three open questions; the plan assumes the answers
per-direction / reuse `ctrl` / quoted numbers, and a different answer changes only
the five keys and the parser's table.
