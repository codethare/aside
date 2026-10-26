# Tasks

## 1. The keyboard, taken once at the map

- [x] 1.1 The surface is created asking for `OnDemand`, and that is the only keyboard-interactivity request the run ever makes
- [x] 1.2 The comment beside it says why: a compositor focuses an interactive layer surface when it maps, and changing the value while mapped makes Hyprland re-run its mouse-move focus and give the keyboard back to the window under the pointer
- [x] 1.3 A test pins the mode
- [x] 1.4 The loop's sleep is the soonest deadline among the cues (`soonest`/`next_wake`), so one cue cannot delay another
- [x] 1.5 An `exclusive`-then-`on-demand` prime was tried and removed: it left the panel on screen and untypable on Hyprland, and the spec says so

## 2. The second summon does what the keyboard says

- [x] 2.1 The control socket means toggle, through a pure `summon_again` that returns `Close`, `Take` or `Nothing`, and the byte it writes says so
- [x] 2.1a `has_keyboard` is tracked from `wl_keyboard.enter`/`leave`, separately from the text input's own state
- [x] 2.1b Taking the keyboard back is an unmap/map of the surface, because river gives a layer surface the keyboard at its map and its WM protocol cannot focus another client's surface
- [x] 2.2 `raise`, `ActivationState`, its handler and its import are deleted
- [x] 2.3 A test covers `summon_again`: it starts a closing, and it does nothing while one is already running

## 3. Docs and gate

- [x] 3.1 README: the second press closes, the keyboard arrives with the panel, and the `layerrule = no_anim, aside` recipe
- [x] 3.2 Man page, `CLAUDE.md` rule 3 and the live checklist say the same
- [ ] 3.3 A hand check: summon, type without touching the pointer; summon again and watch it close; focus another window while it is up and type there
- [x] 3.4 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`
