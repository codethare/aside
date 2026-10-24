# Tasks

## 1. The interactivity

- [x] 1.1 `INTERACTIVITY = KeyboardInteractivity::Exclusive`, with the reason beside it, and the layer surface asks for it
- [x] 1.2 A test fails if it goes back to `OnDemand`

## 2. Docs and gate

- [x] 2.1 README, man page, live checklist and `CLAUDE.md` say the panel has the keyboard when it appears, and that `Esc` gives it up
- [x] 2.2 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`, `--preview`
