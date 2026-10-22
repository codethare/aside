# Proposal

## Why

The thinking ball costs the panel its shape: the bar has to shrink to a 60 px
circle and come back, so a question cannot be typed or corrected while the answer is
being fetched, and the field the user is looking at disappears at the moment they
send. A running light around the field says "working" without taking anything away.

## What Changes

- **The thinking state is the bar itself**, still 640 × 30, still showing what was
  typed. The 60 × 60 circle and its four blinking squares are gone.
- **A light travels the bar's rim**: a short bright segment runs around the card's
  rounded outline, one lap in 1.4 s, over the rim's own lit and dark profile. It is
  the working indicator, and it stops when the answer's first text arrives.
- The ball's morphs go with the ball: `Idle` and `Thinking` are the same shape, so
  sending a question changes no geometry at all. Only the bar's own light moves.
- Typing stays ignored while the request is out, as before: only `Esc` means
  anything, and the typed question is still visible under the light.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the thinking state becomes the bar with a light on its rim; the
  three-state table loses the ball, so the panel has two sizes rather than three.

## Impact

- `src/main.rs`: the thinking phase's size, the light's clock, the morph that no
  longer happens between the bar and the ball.
- `src/panel.rs`: the ball painters go; an orbit painter joins the rim.
- `src/render.rs`: the outline walk the light follows.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`, `CLAUDE.md` rule 4.
- No new dependency, no new file, nothing on disk.
