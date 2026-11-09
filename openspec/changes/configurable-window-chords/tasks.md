# Tasks

TDD throughout: the named test is written and run before the code that makes it
pass, and the run is read. Gate per task: `cargo test <name>` green, then the whole
suite.

## 1. A chord is a value

- [ ] 1.1 `Chord { alt, ctrl, shift, keysym }` and `Chords { left, right, up, down, step, far }` in `main.rs`, with `Chords::default()` = today's `alt+hjkl`, 16, 160
- [ ] 1.2 `fn parse_chord(&str) -> Result<Chord, String>`: modifiers then one key, case-insensitive, spaces tolerated; keys `a..z`, `0..9`, the four arrows, `pageup`, `pagedown`, `home`, `end`
- [ ] 1.3 Test `a_chord_is_a_modifier_and_a_key`: `"alt+h"`, `"Alt + Ctrl + L"`, `"shift+left"` parse; `"h"`, `"alt+super+h"`, `"alt+h+l"`, `"alt+?"` each return an error
- [ ] 1.4 Test `a_chord_without_a_modifier_is_refused` and `a_chord_with_two_keys_is_refused`: the message says what is wrong

## 2. The config answers, with file and line

- [ ] 2.1 `Chords::from_config(&[(String, String)]) -> Result<Chords, ConfigError>` reading `window_left`, `window_right`, `window_up`, `window_down`, `window_step`
- [ ] 2.2 `window_step` bounded to `1..=512`; outside it the file and the line are named
- [ ] 2.3 A chord that repeats a direction, or that equals `alt+c`, `ctrl+c`, `ctrl+v`, `ctrl+t`, is refused with the file and the line
- [ ] 2.4 Test `the_step_is_bounded`: `"0"`, `"513"`, `"sixteen"` refused; `"24"` accepted
- [ ] 2.5 Test `the_defaults_are_todays_behaviour`: an empty config gives `alt+hjkl`, 16, 160
- [ ] 2.6 Test `two_directions_on_one_chord_are_refused`: the second line is the one named

## 3. The key path takes the chords

- [ ] 3.1 `key_action(&Chords, keysym, alt, ctrl, shift, has_text) -> Key`; `Key::Window { dx, dy }` carries the delta, `window_move` folds into `Chords::delta`
- [ ] 3.2 `shift` comes from the seat's `Modifiers`, as `alt` and `ctrl` already do
- [ ] 3.3 `State` carries the resolved `Chords`; `main` resolves them before the window opens, and a bad one exits like a bad `font_size`
- [ ] 3.4 Test `a_configured_chord_moves_the_panel`: `ctrl+shift+h` with `window_step = "24"` gives `(-24, 0)`, and with `ctrl` added gives `(-240, 0)`

## 4. Copy mode keeps the chord before its own table

- [ ] 4.1 `on_copy_key` asks `Chords::delta` first, as it does today, and returns without touching the cursor
- [ ] 4.2 Test `copy_mode_takes_the_chord_before_its_own_table`: the delta for a chord is the window move, while the bare letter stays a cursor motion
- [ ] 4.3 Test that a configured chord is honoured in typing mode too, and that its letters still type when the modifier is absent

## 5. Docs and gate

- [ ] 5.1 `README` (the config section: six keys become eleven, with an example) and `docs/aside.1` CONFIGURATION
- [ ] 5.2 `CLAUDE.md` rule 3: the chords are the config's, `Alt+hjkl` is the default
- [ ] 5.3 `docs/live-checklist.md`: a configured chord moves the panel in typing and in copy mode; a modifier-less chord refuses to open the panel and names the line
- [ ] 5.4 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`, `cargo audit`
