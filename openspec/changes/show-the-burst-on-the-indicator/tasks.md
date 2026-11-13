# Tasks

## 1. The number on the strip

- [x] 1.1 `burst_label`: the multiplier as `1.5x`, `2x`, `3.5x`, `4x`, one decimal only when it is not whole — verify: a test pins each of those strings, and `1x` for the strip's own case
- [x] 1.2 While the multiplier stands above one, the number is drawn on the strip: centred on the strip's own column and on the thumb's middle, over the strip, clamped inside the card's right edge and rows, set smaller than the panel's text — verify: a panel frame test finds the same three accent columns and the same rows with and without a burst (the strip keeps its shape), the pixels the burst adds inside the card and inside its rows, those glyphs no taller than a line of the label's own face and shorter than a row of the answer, the number straddling the strip's column, and a wider label for `2.5x` than for `4x`
- [x] 1.3 `live_factor`: the multiplier as it stands now, one once 400 ms have passed without a step — verify: a test asserts 4x just after a step, 1x at the window, 1x with no step at all, and 1x for a burst of one
- [x] 1.4 The label's own face: `LABEL_SHARE` of the panel's size, read on its own thread beside the panel's font at startup and again on a text-size change, failing soft to no number — verify: the painter takes `Burst { factor, font }`, so a frame without that face draws no number at all, and the frame test paints with the small face
- [x] 1.5 A burst of one step changes nothing: the frame with a 1x burst is the frame with none, byte for byte — verify: the same frame test asserts it

## 2. Docs

- [x] 2.1 `README.md`, `docs/aside.1`, `docs/live-checklist.md`: the strip carries the multiplier in small type, and the number goes with the burst — verify: none of the three still says capsule, and each names the number, its size and when it goes

## 3. Integration

- [ ] 3.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean. Waiting on a machine with the system faces installed: this sandbox has no monospace face and no CJK face, so four font-metric tests fail before and after the change, and they are the only failures
- [ ] 3.2 Run a real session — verify: one notch shows the plain strip with no number, a spin writes `1.5x` and on up to `4x` on the strip at the thumb's middle, the number goes 400 ms after the last notch, it never crosses the card's right edge at either end of the track, and the answer's lines read as they did before
