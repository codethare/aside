# Tasks

## 1. A fixed box that scrolls

- [x] 1.1 Size the answering panel from the anchored gap and the row cap alone, with no term for the content or the drag — verify: a test asserts one line and a thousand lines give the same panel size, and that a drag offset does not change it
- [x] 1.2 Make the replies card the room's full height and place each line by `row.y - viewport.top` — verify: a drawing test with a tall transcript finds the newest lines inside the card and none outside it
- [x] 1.3 Keep the reveal, the scroll indicator and `MAX_ROWS` working against the fixed box — verify: the reveal test still passes at half height and the thumb appears only when the transcript overflows

## 2. Movement on both axes

- [x] 2.1 Anchor the surface to the bottom and left edges — verify: a real session moves the panel sideways with a drag and with `l`, and it keeps its size
- [x] 2.2 Keep the panel centred by its own left margin now that the compositor no longer centres it — verify: a fresh run places it centred, and it cannot be dragged off either side

## 3. Docs

- [x] 3.1 Update `README.md`, `docs/aside.1` and `docs/live-checklist.md` for the fixed box and free movement — verify: none still says the panel grows with the answer or that it only moves vertically

## 4. Integration

- [x] 4.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [ ] 4.2 Run a real session — verify: a long answer stays inside a fixed box and pages, dragging and `hjkl` move it in both axes, and its size never changes while it moves
