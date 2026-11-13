# Tasks

## 1. The capsule

- [x] 1.1 `burst_label`: the multiplier as `1.5x`, `2x`, `3.5x`, `4x`, one decimal only when it is not whole — verify: a test pins each of those strings, and `1x` for the strip's own case
- [x] 1.2 While the multiplier stands above one the indicator is a capsule: accent fill, radius half its height, the thumb's place, at least a row tall, clamped to the card's rows, right-aligned inside the card, no narrower than its own text — verify: a panel frame test finds the strip 3 px wide for one notch and a pill wider than 20 px for a burst; it finds no ink at the pill's own top-left corner (a capsule, not a box), and pixels inside the pill that are not the fill (the number); and a wider label is a wider pill
- [x] 1.3 `live_factor`: the multiplier as it stands now, one once 400 ms have passed without a step — verify: a test asserts 4x just after a step, 1x at the window, 1x with no step at all, and 1x for a burst of one
- [x] 1.4 Hand it to the painter from `State` — verify: the panel's own tests paint both ways, and 1.2 covers the drawn result

## 2. Docs

- [x] 2.1 `README.md`, `docs/aside.1`, `docs/live-checklist.md`: the indicator becomes a capsule with the multiplier on it while a burst accelerates, and returns to the strip — verify: each names the capsule and the number, and the checklist has a step a person can run with a wheel

## 3. Integration

- [ ] 3.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean. Waiting on a machine with the system faces installed: this sandbox has no monospace face and no CJK face, so four font-metric tests fail before and after the change, and they are the only failures
- [ ] 3.2 Run a real session — verify: one notch shows the plain strip, a spin turns it into a capsule reading `1.5x` and on up to `4x`, it returns to the strip 400 ms after the last notch, and the capsule stays inside the card at both ends of the track
