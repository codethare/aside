# Proposal

## Why

On a 150 % output the panel is drawn at the integer `wl_output.scale` the
compositor reports — usually 1 — and the compositor upscales the buffer to 1.5×,
so every glyph and the rim are soft. A modern Wayland desktop exposes
`wp_fractional_scale_v1` and `wp_viewporter` for exactly this; the panel binds
neither, and the README lists the omission as a known limit.

## What Changes

- Bind `wp_viewporter` and `wp_fractional_scale_v1` when the compositor has them.
  The panel asks the fractional-scale object on its surface for a preferred scale.
- On a preferred scale of `p/120`, render at `ceil(p/120)` buffer pixels per
  logical pixel, keep `wl_surface.buffer_scale` at 1 as the protocol asks, and set
  the viewport destination to the surface's logical size. The compositor scales the
  larger buffer down to that destination, then to the physical scale: text keeps
  its logical size, and 1.5× is drawn from 2× pixels instead of upscaled from 1×.
- Recompute the font, rows and buffer when the preferred scale changes while the
  panel is open, exactly as an integer scale change does.
- Without the protocol, the existing integer `wl_output.scale` behaviour is kept,
  and the viewport destination is unset.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: 1.5× and other fractional scales are rendered from a larger buffer
  through the fractional-scale and viewporter protocols; integer scale remains the
  fallback.

## Impact

- `Cargo.toml`: enable `wayland-protocols`' `staging` feature (the module still
  ships there); no new crate.
- `src/main.rs`: two optional globals, a viewport and a fractional-scale object on
  the surface, and one scale-adoption path shared with `wl_surface.scale`.
- `README.md`, `CLAUDE.md`, `docs/aside.1` when the code lands.
- Nothing on disk; no change to sandbox or provider.
