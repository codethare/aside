# Tasks

## 1. Per-keystroke arbitration

- [x] 1.1 Add `hold`, `claimed` and `flush` to `Ime` — verify: tests cover silence (the key is typed), a pre-edit in the round (the key is dropped), a commit in the round (only the commit is typed), and no input method at all (keys type immediately)
- [x] 1.2 Hold text keys in `main::on_key` while an input method is in play and flush them in `tick` — verify: with the input method silent, typed text reaches the input line in the same frame

## 2. Docs

- [x] 2.1 State the arbitration in `README.md` where the input method is described — verify: it says a key is held for one round and typed when the input method does not claim it

## 3. Integration

- [x] 3.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [ ] 3.2 Run a real session — verify: type Chinese with fcitx, switch it off, then type letters and digits — they appear; switch it back on and compose again — the composition still works and nothing is duplicated
