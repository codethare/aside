# Tasks

## 1. Only the current turn is laid out

- [x] 1.1 Give the transcript one rule: a turn that is not the current one keeps its text for the request and is never laid out — verify: tests assert that after a second question only the second answer has rows, height and scroll, that the earlier entries keep their text, and that the request still carries them in order
- [x] 1.2 Reset the block chain when a question is asked, and start `rewrap` at the current turn — verify: after a resize the box still holds the current turn alone, at offset 0
- [x] 1.3 Keep the highlight, paging and copy inside the current turn — verify: the row indices after two turns address the second answer's lines only

## 2. Docs

- [x] 2.1 Update `README.md`, `docs/aside.1` and `docs/live-checklist.md` — verify: each says only the current turn is on screen and that earlier turns still go to the provider

## 3. Integration

- [x] 3.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [ ] 3.2 Run a real session — verify: a follow-up replaces the answer on screen, and the model still answers with the earlier exchange in mind
