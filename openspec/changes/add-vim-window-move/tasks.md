# Tasks

## 1. One clamped move

- [x] 1.1 Route the drag and the keyboard through `move_panel`, which clamps both margins to the output and keeps the drag offset equal to the gap on screen — verify: a test asserts a step cannot leave the output on any side, and that the offset and the painted margin agree
- [x] 1.2 Add the vertical clamp the drag was missing — verify: a test drags far past the top and the surface's bottom margin stops where the surface still fits

## 2. The vim keys

- [x] 2.1 Move `hjkl` to window steps of 16 px, ten times that with `Ctrl` — verify: a pure test covers the four directions with and without `Ctrl`, and that no other key moves the window
- [x] 2.2 Move the transcript's step and ends to `Ctrl+n`/`Ctrl+p` and `g`/`G` — verify: the vim table test covers every key, and the previous `j`/`k`/`h`/`l` transcript meanings are gone
- [x] 2.3 Keep the cap in step with a keyboard move — verify: stepping up raises the cap, and the panel never exceeds the room above it

## 3. Docs

- [x] 3.1 Update the key tables in `README.md`, `docs/aside.1` and `docs/live-checklist.md` — verify: each names the four window steps, the `Ctrl` larger step, and the transcript's new keys, and none still says `j`/`k` move the highlight

## 4. Integration

- [x] 4.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [ ] 4.2 Run a real session — verify: `Ctrl+T [`, then `hjkl` moves the panel and `Ctrl+hjkl` moves it further, `n`/`p`/`g`/`G` still read the transcript, and the panel stops at the screen edges
