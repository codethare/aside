# Tasks

## 1. The outline and the light

- [x] 1.1 `render::outline` walks a card's rim in order — verify: a test asserts the polyline is closed, has no gaps, and every point is on the card's own span
- [x] 1.2 An orbit painter lights the segment behind the head, two pixels wide, over the rim — verify: a test paints two heads half a lap apart and finds the lit pixels on opposite sides, and one that the rim's untouched pixels are unchanged
- [x] 1.3 `orbit_at` reads the head off the clock at one lap per 1.4 s — verify: a test asserts it wraps, is monotonic inside a lap, and that a late frame lands where the time says

## 2. The bar is the thinking state

- [x] 2.1 Delete `State::Ball`, `ball_card`, `draw_ball_squares`, `blink_at` and the bar→ball morph — verify: the crate builds with no dead code, and `size_for(Thinking)` equals the idle bar's size
- [x] 2.2 Paint the light while a request is open, over the bar's own rim, with the input line still drawn — verify: a frame test finds the typed text and the light together, and no light once the answer has started
- [x] 2.3 Start a morph only when the shape changes — verify: a test asserts idle→thinking needs none and thinking→answering still does

## 3. Docs

- [x] 3.1 Update `README.md`, `docs/aside.1`, `docs/live-checklist.md` and `CLAUDE.md` rule 4 — verify: none mentions a ball or four blinking squares, and each names the rim light and the two sizes

## 4. Integration

- [x] 4.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [ ] 4.2 Run a real session — verify: sending a question keeps the bar still and lights its rim, the light laps smoothly, and it is gone when the answer starts
