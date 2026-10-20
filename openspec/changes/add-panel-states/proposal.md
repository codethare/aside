## Why

The panel that ships now is a 900×520 window with a bar at its bottom: it looks
nothing like `wlmenu`, and it is either huge and empty or scrolling inside a
fixed frame. A summoned assistant should be a bar first, take the screen only
while it has an answer to show, and say "working" without moving anything.

## What Changes

- The window becomes a `wlr-layer-shell` surface: overlay layer, anchored to the
  top edge, 32 px margin, horizontally centred, exclusive keyboard. The same
  dependency set already provides it (`smithay_client_toolkit::shell::wlr_layer`).
- The bar is `wlmenu`'s bar: 640 wide, 20 px text, 30 px tall, `PAD` 16,
  radius 12, and it sits at the **top** of the surface with the transcript below.
- The frame is `wlmenu`'s frame: a premultiplied translucent body inside a 24 px
  shadow margin, a hairline border, antialiased corners, and a 3 px scroll
  indicator while the transcript overflows. The surface grows by the shadow
  margin, so the panel keeps its 32 px visible gap.
- Three states replace the fixed frame:
  - **idle** — the bar alone, 640×24;
  - **thinking** — the panel shrinks to a 48×48 square (`2 × bar_h`, centred)
    and paints a filled circle with four 4 px squares in a 2×2 grid, one lit at
    a time, 200 ms apart, 800 ms a cycle;
  - **answering** — back to 640 wide, height grows with the transcript and is
    capped at `output_height − 32 (top) − 32 (bottom breathing room)`; anything
    longer scrolls inside.
- Typing is ignored while thinking; `Esc` still cancels. Every other key keeps
  its meaning.
- The panel is no longer resized by the compositor: it sizes itself, and a size
  change still re-wraps exactly once.

Out of scope: height tweening (states snap), fractional scaling, and any
compositor without `wlr-layer-shell` — such a session now fails at startup
instead of opening a floating window.

## Impact

- `src/main.rs`: xdg-shell out, layer-shell in; a `Phase` enum; own sizing with
  an output-height cap; the configure wait from the previous change applies
  unchanged.
- `src/panel.rs`: bar on top instead of the bottom, plus the ball and its
  squares as a third drawing path.
- `src/font.rs`: text size 13 → 16 (`wlmenu`'s), which moves every row height.
- README, man page and `CLAUDE.md`'s token line follow the new geometry.
