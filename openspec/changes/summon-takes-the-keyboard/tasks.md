# Tasks

## 1. A bounded prime instead of an endless grab

- [x] 1.1 `PRIME` (`Exclusive`) and `PRIME_MS` (75) sit beside `INTERACTIVITY` (`OnDemand`), each with the reason it is what it is
- [x] 1.2 The surface is created asking for `Exclusive`; the release to `OnDemand` happens in `tick`, no later than `PRIME_MS` after the first configure, and immediately once the panel starts closing
- [x] 1.3 The event loop sleeps to the prime's deadline instead of overshooting it
- [x] 1.4 A test pins the shape of the contract: the settled mode is `OnDemand`, the prime is `Exclusive`, and it is bounded
- [x] 1.5 The loop's sleep is the soonest deadline among the cues (`soonest`/`next_wake`), so the prime cannot be delayed behind a copy note or any other sleep

## 2. The second summon puts it away

- [x] 2.1 The control socket means toggle, through a pure `summon_again`, and the byte it writes says so
- [x] 2.2 `raise`, `ActivationState`, its handler and its import are deleted
- [x] 2.3 A test covers `summon_again`: it starts a closing, and it does nothing while one is already running

## 3. Docs and gate

- [x] 3.1 README: the second press closes, the keyboard arrives with the panel, and the `layerrule = no_anim, aside` recipe
- [x] 3.2 Man page, `CLAUDE.md` rule 3 and the live checklist say the same
- [ ] 3.3 A hand check: summon, type without touching the pointer; summon again and watch it close; focus another window while it is up and type there
- [x] 3.4 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`
