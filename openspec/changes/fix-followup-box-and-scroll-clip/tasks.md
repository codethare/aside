# Tasks

## 1. The follow-up keeps the box

- [x] 1.1 Draw a request in flight as the shape already on screen — verify: a test covers the shape rule for every (current shape, phase) pair, and a follow-up's shape is the panel's
- [x] 1.2 Keep the working light on the bar's rim while the box stays up — verify: a frame test finds the light's ink in a settled panel frame while a request is out

## 2. Whole rows only

- [x] 2.1 Drop a row that starts above the card instead of drawing part of it — verify: a frame test with a half-row offset finds no ink in the card's top row band from the row above, and every fully visible row is unchanged

## 3. Docs

- [x] 3.1 `README.md`, `docs/aside.1`, `docs/live-checklist.md` — verify: each says a follow-up keeps the box and that the light runs on the bar's rim

## 4. Integration

- [x] 4.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [ ] 4.2 Run a real session — verify: a second question does not shrink the panel, and scrolling never squashes two lines together
