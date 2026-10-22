# Proposal

## Why

The panel snaps between its three states: it appears already finished, the bar
becomes a ball in one frame, and the ball becomes the answering panel in another.
Each state is a different shape of the same glass, so the change should read as that
shape changing rather than as three windows.

## What Changes

- **The panel rises into place** when it opens: the bar's card grows up out of the
  bottom edge over 120 ms, its text fading in behind it.
- **The bar becomes the ball**: one card shrinks from the bar's band into the 60 px
  circle over 200 ms on a spring, its width, height and corner radius interpolated,
  the bar's text fading out and the ball's squares fading in.
- **The ball becomes the answering panel**: the same card grows from the circle into
  the panel over 240 ms, the replies card fading up inside it and the bar's own line
  fading in at the end.
- Every transition interpolates inside **one** buffer at the larger of the two
  states' sizes, so the surface is resized once per transition and never per frame.
  A settled frame is painted by exactly the painters it is painted by today.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: entering, bar-to-ball and ball-to-panel are animated shapes of one
  card rather than three stateless frames.

## Impact

- `src/main.rs`: the phase change becomes a morph, the entrance clock, and the
  union size a transition needs.
- `src/panel.rs`: a `Look` (state, where it came from, progress) and one morph
  painter; the settled painters take a fade of 1.
- `src/render.rs`: `dim`, one helper for a colour at a fraction of its alpha.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`.
- No new dependency, no new file, nothing on disk.
