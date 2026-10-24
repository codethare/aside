# Tasks

## 1. The keyboard is the compositor's to move

- [x] 1.1 `INTERACTIVITY` is `OnDemand`, with the reason beside it (exclusive defeats every view focus)
- [x] 1.2 The old test becomes `the_panel_leaves_the_keyboard_shareable`
- [x] 1.3 Losing focus still disables the text input and drops a pre-edit; regaining it enables the input again

## 2. Docs and gate

- [x] 2.1 README, man page, `CLAUDE.md` rule 3 and the live checklist say what the session does now
- [x] 2.2 A hand check: with the panel up, focus another window and type into it; then focus the panel and type into it
- [x] 2.3 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`
