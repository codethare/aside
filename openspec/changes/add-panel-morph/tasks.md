# Tasks

## 1. The clock

- [x] 1.1 Add `morph_at` (spring, normalised) and `enter_at` (ease-out), both read off elapsed milliseconds — verify: tests assert 0 at 0, exactly 1 at the deadline, monotonic, and bounded

## 2. The shapes

- [x] 2.1 `panel::State`, `panel::Look` and a `card_of` that gives each state's card from the panel — verify: a test asserts the three cards' geometry (the bar's band at the bottom, the ball's square centred, the pane's rounded panel)
- [x] 2.2 One morph painter: lerp the two cards, paint shadow and body, draw the from-content at `1 - t` and the to-content at `t`, clipped to the card — verify: at `t = 0` the frame matches the from-state's card geometry and at `t = 1` the to-state's
- [x] 2.3 `render::dim`, and the `fade` argument on the bar, replies and ball painters with 1.0 for every settled call — verify: a settled frame with a `t = 1` look is byte-identical whatever `from` says

## 3. The state machine

- [x] 3.1 `set_phase` starts a morph, and `target_size` reports the union of the two states while it runs — verify: `size_for` tests cover the bar/ball union and that the panel is unchanged
- [x] 3.2 Settle the morph when it ends and set the surface once — verify: a test asserts the surface size after settling equals `size_for(target)`
- [x] 3.3 Start the replies reveal when the morph ends, not on top of it — verify: the reveal test covers a morph that is still running and asserts the reveal has not started
- [x] 3.4 The entrance on the first frame, and typing during it unchanged — verify: a test asserts the first frame's card is shorter than the bar and the last equals the idle bar

## 4. Docs

- [x] 4.1 Update `README.md`, `docs/aside.1` and `docs/live-checklist.md` — verify: each names the three transitions and their durations
- [x] 4.2 Open as a line spreading out of a point and unfold up out of it; close as that run backwards — verify: `enter_tracks`/`close_tracks` tests cover both tracks, their order and their ends, and a frame test finds a point, then a line, then the bar

## 5. Integration

- [x] 5.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [ ] 5.2 Run a real session — verify: opening rises, `Enter` shrinks the bar into the ball, the first text grows it into the panel, and no frame resizes the surface twice
