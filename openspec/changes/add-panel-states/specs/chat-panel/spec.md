# Spec Delta

## Purpose

The panel becomes a summoned bar that grows only as far as it has something to
show: a `wlmenu`-sized bar at the top of the screen, a thinking ball while the
answer is being fetched, and a transcript panel below it afterwards.

## ADDED Requirements

### Requirement: The panel is a layer surface in three states

The panel SHALL be a `wlr-layer-shell` surface on the overlay layer, anchored to
the top edge with a 32 px margin, horizontally centred on its output, with
exclusive keyboard interactivity. It SHALL have exactly three states:

| State | Panel size (logical px, 1×) | Content |
|---|---|---|
| idle | 640 × 30 | the input bar alone |
| thinking | 60 × 60 | a filled circle with four blinking squares |
| answering | 640 × (24 + 1 + rows × row_h), capped | the bar on top, the transcript below it |

`row_h` is `row_h_for(20)` (30 at 1×) and `rows × row_h` is the transcript's own
height. A size change SHALL be committed before the frame that uses it. These are
the panel's sizes: the surface carries the frame margin below, so its size is
`panel + 2 × 24` on each axis and its top margin is 8 px — the panel keeps a
32 px visible gap to the screen edge.

#### Scenario: Summoned idle

- **WHEN** the panel opens with nothing asked
- **THEN** only the bar is on screen: a 640 × 30 panel, no transcript area, inside a 688 × 78 surface

#### Scenario: Enter starts the ball

- **WHEN** the user presses `Enter` with a non-empty input
- **THEN** the request starts and the panel becomes a 60 × 60 square centred on the bar, painting a circle

#### Scenario: The first delta expands the panel

- **WHEN** the answer's first text arrives
- **THEN** the panel returns to 640 wide, keeps the bar as its top row, and grows by one row per transcript row

#### Scenario: Typing during the ball is ignored

- **WHEN** the panel is thinking and a printable key is pressed
- **THEN** the input is unchanged, and `Esc` still cancels the request

### Requirement: The bar matches wlmenu's bar

The bar SHALL use `wlmenu`'s metrics unchanged: width 640, text size 20 px,
row height `row_h_for(20)`, `PAD` 16, corner radius 12, an 8 px gap after a 4 px
`LABEL_ACCENT` rail, and a 1 px divider at 25 % alpha separating it from the
transcript. The bar SHALL be the top row of the surface, with the transcript
below it, because the surface grows downwards from a top anchor.

#### Scenario: One bar, two products

- **WHEN** the idle panel and `wlmenu`'s bar are compared at the same scale
- **THEN** their widths, heights, padding, radius, rail and divider are identical

### Requirement: The frame matches wlmenu's frame

Outside the bar, the panel SHALL be drawn exactly as `wlmenu` draws its panel: a
premultiplied translucent body (`BG_NORMAL` at 83 % alpha, `BG_PROMPT` at 86 %),
a 24 px shadow margin around it holding both drop-shadow layers, a one-pixel
hairline border lit at the top and dark underneath, antialiased corners, the
divider inset by `PAD`, the selection band inset one pixel inside the border,
and a 3 px `LABEL_ACCENT` scroll indicator at the right edge while the
transcript overflows the viewport. The thinking ball SHALL use the same body,
border and shadow inside its circle.

#### Scenario: One panel, two products

- **WHEN** a screenshot of `aside`'s answering panel and one of `wlmenu` are laid side by side
- **THEN** their shadow falloff, body translucency, border, corner shape, divider inset and selection inset agree

#### Scenario: A long transcript scrolls with an indicator

- **WHEN** the transcript has more rows than the capped panel shows
- **THEN** a 3 px `LABEL_ACCENT` thumb sits at the right edge, its length in proportion to the visible rows

### Requirement: The thinking ball blinks without moving

The ball SHALL be a filled circle of diameter `2 × row_h` painted in `BG_PROMPT`
on the cleared surface, and SHALL contain four `4 px` squares in a `2 × 2` grid
with `2 px` gaps and `4 px` margins, in `FG_PROMPT`. One square SHALL be lit at a
time, advancing every `200 ms` and returning to the first after `800 ms`. The
ball's geometry SHALL NOT change while it is displayed.

#### Scenario: The blink moves, the ball does not

- **WHEN** two frames `200 ms` apart are painted
- **THEN** the lit square differs and the circle's pixels are identical

#### Scenario: Phase comes from the clock, not from frame count

- **WHEN** the loop wakes late, so several steps have elapsed
- **THEN** the lit square matches the elapsed time rather than the number of frames painted

### Requirement: The answer's height is capped by the output

The surface height in the answering state SHALL NOT exceed
`output_height − 32 (top margin) − 32 (bottom breathing room)` for the panel
itself; the shadow margin hangs inside that gap. When the transcript needs more,
the panel SHALL stay at the cap and scroll inside it, keeping the existing
highlight and `PageUp`/`PageDown`/`Alt+C` behaviour.

#### Scenario: A long answer hits the cap and scrolls

- **WHEN** the transcript is taller than the cap
- **THEN** the surface stops at the cap, the newest rows stay visible, and scrolling reaches the older ones

#### Scenario: No output height yet

- **WHEN** the output's logical height is not known yet
- **THEN** the panel uses the cap of a 1080-pixel output and re-sizes once the real height is known

## MODIFIED Requirements

### Requirement: The panel is an ordinary window, summoned by a keybinding

The panel SHALL be summoned by a compositor keybinding that runs `aside`, and
SHALL take keyboard focus on appearing. It is a layer surface, not an ordinary
window: the compositor places it by the anchor and margin above, and it cannot
be moved, tiled or decorated by the user. When the compositor closes the surface
or the connection, the process SHALL exit as the failure rules say: reported and
non-zero before the panel appeared, quietly after it did.

#### Scenario: Panel appears focused

- **WHEN** the keybinding runs `aside` with a valid credential
- **THEN** the bar appears at the top of the output, centred, and receives typed text

#### Scenario: No layer-shell support

- **WHEN** the compositor does not advertise `zwlr_layer_shell_v1`
- **THEN** `aside` reports that it cannot place its panel and exits non-zero without showing a window

### Requirement: The window can be resized and re-wraps once per resize

The panel sizes itself. A change in the surface size — a new state, a growing
transcript, a new output geometry or scale — SHALL re-wrap the transcript exactly
once for that width, and shall NOT re-wrap on every frame.

#### Scenario: Growing the panel re-wraps once

- **WHEN** the answering state widens or narrows the surface
- **THEN** the transcript is wrapped once at the new width and every later frame reuses it

#### Scenario: A very tall window

- **WHEN** the cap allows more rows than `MAX_ROWS`
- **THEN** the viewport still stops at `MAX_ROWS` and scrolling covers the rest
