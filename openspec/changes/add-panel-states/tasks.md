# Tasks

## 1. Layer-shell surface

- [x] 1.1 Replace the `xdg-shell` window with a `wlr-layer-shell` surface: overlay layer, `Anchor::TOP`, margin 32, exclusive keyboard, horizontally centred on the output — verify: `aside` appears as a top bar under sway and river, and the configure wait still applies (no buffer before the first configure)
- [x] 1.2 Fail visibly when the protocol is missing — verify: a compositor without `zwlr_layer_shell_v1` makes `aside` exit non-zero with one line, rather than opening a window or idling

## 2. Bar metrics

- [x] 2.1 Match `wlmenu`: width 640, text size 20, `row_h_for(20)`, `PAD` 16, radius 12, rail/gap/divider unchanged — verify: the idle bar's width and height match `wlmenu`'s at 1×
- [x] 2.2 Move the bar to the top row and the transcript below it — verify: the bar stays put while the panel grows downwards, and the divider sits under the bar

## 3. The three states

- [x] 3.1 Add the state machine: idle → thinking on `Enter`, thinking → answering on the first delta, answering stays until `Esc` — verify: each transition resizes the surface once and commits a frame at the new size
- [x] 3.2 Thinking ball: 48×48 surface (`2 × row_h`), filled circle, four 4 px squares in a 2×2 grid, one lit at a time every 200 ms over an 800 ms cycle, phase from the clock — verify: consecutive frames differ only in which square is lit, and a late frame lands on the correct square
- [x] 3.3 Ignore typing while thinking, keep `Esc` working — verify: printable keys leave the input untouched during the ball

## 4. Height cap and scrolling

- [x] 4.1 Track the output's logical height and cap the answering surface at `output_h − 64` — verify: a long answer stops at the cap and the newest rows stay visible
- [x] 4.2 Fall back to a 1080-pixel output until the real height is known, then re-size once — verify: no frame is painted at an unbounded height, and the cap tightens when the output reports
- [x] 4.3 Keep `Up`/`Down`/`PageUp`/`PageDown`/`Alt+C` working inside the cap — verify: the highlight scrolls the viewport at the cap exactly as it did in the fixed frame

## 5. Docs and gate

- [x] 5.1 Update README, man page and `CLAUDE.md`'s token line for the layer surface and the three states — verify: each document names the width, the bar height, the state sizes and the cap formula
- [x] 5.2 Run the gate and a real session — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` clean, and a headless sway and river run each show the bar, the ball and an expanded panel

## 6. Frame parity with wlmenu

- [x] 6.1 Copy the frame: 24 px shadow margin, premultiplied translucent body, hairline border, antialiased corners, divider and selection insets — verify: a rendered panel's margin holds a black shadow, its body alpha is `wlmenu`'s, and an edge pixel is partially covered
- [x] 6.2 Grow the layer surface by the shadow margin and keep the panel's visible gap at 32 px — verify: the idle surface is 688 × 78 with an 8 px top margin, and the panel is still 640 × 30
- [x] 6.3 Draw the scroll indicator while the transcript overflows — verify: a 3 px `LABEL_ACCENT` thumb appears only when the rows exceed the viewport and moves with `viewport.top`
- [x] 6.4 Update README, man page and the token line with the frame — verify: each document names the shadow margin, the translucency and the indicator
- [x] 6.5 Run the gate and look at `--preview` — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` clean, and the preview shows the shadow and the border
- [x] 6.6 Follow `wlmenu`'s body size to 20 px (15 pt) — verify: `font::SIZE` is 20, the bar is `row_h_for(20)` = 30 px, and the thinking ball is 60 × 60
