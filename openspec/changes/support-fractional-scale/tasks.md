# Tasks

## 1. Bind and ask

- [x] 1.1 Bind `wp_viewporter` and `wp_fractional_scale_v1` optionally; create a
  viewport and a fractional-scale object for the layer surface when both exist —
  verify: a headless sway session starts `aside` and keeps it alive with no protocol
  error in the compositor log, and a compositor without the globals still starts
  (the binds are `.ok()`)
- [x] 1.2 Handle `preferred_scale`: `p/120` chooses `ceil(p/120)` buffer pixels per
  logical pixel, leaves `buffer_scale` at 1, and keeps the viewport destination at
  the logical surface size — verify: a pure test pins the rounding for 120, 180,
  240, 150 and 90; a live 1.5x output check is 3.2
- [x] 1.3 The viewport destination is set after every `apply_size` and unset when no
  fractional preference is in use — verify: the code path is one `sync_viewport`,
  called from `apply_size`, `adopt_scale` and `preferred_scale`; the no-protocol
  headless run exercises the unset branch
- [x] 1.4 The per-surface scaling objects are made only after `boot` has settled the
  final layer surface, and are destroyed before a surface is replaced — verify: the
  first build made them for the compositor's default surface, which `boot` then
  replaced for the only output, and `set_destination` on the dead surface was a
  protocol error; after moving the creation into `boot`, headless sway stays up and
  logs no protocol error both with and without `--output`

## 2. Adopt the scale once

- [x] 2.1 One `adopt_scale` path serves the integer `wl_output.scale` change and the
  fractional preferred scale: reload the font, rewrap, resize the viewport height,
  drop the buffer, mark dirty; a fractional preference outranks later integer
  events — verify: the existing scale/re-wrap tests pass, and `scale_factor_changed`
  returns while `fractional_pref` is set
- [x] 2.2 The fractional handler sets `buffer_scale` to 1 and marks a frame even when
  the integer scale did not change — verify: `preferred_scale` sets dirty after
  `adopt_scale`

## 3. Integration

- [x] 3.1 Run the gate — verify: 263 tests pass, only the four pre-existing
  font-environment failures fail, `cargo fmt --check` and `cargo clippy --all-targets
  -- -D warnings` are clean
- [ ] 3.2 A live 1.5x session — verify: text has the same logical size as at 1x, no
  upscaled softness, and a scale change while open re-wraps once
