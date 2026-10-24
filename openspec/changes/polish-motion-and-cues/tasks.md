# Tasks

## 1. A shape change lands on its deadline

- [x] 1.1 `morph_at` is the ease-out cubic, `track_at`; the spring, its damping, its frequency, its normalisation and the clamp go
- [x] 1.2 The clock test gains "the value never exceeds 1", and the durations are unchanged

## 2. The exit is shorter than the entrance

- [x] 2.1 `CLOSE_MS` is 200, with the reason beside it
- [x] 2.2 `close_tracks` and `TRACK_SHARE` are untouched, and the existing half-way and nine-tenths assertions still hold

## 3. The light waits for a slow request

- [x] 3.1 `LIGHT_AFTER_MS = 500` and a pure `light_at(elapsed) -> Option<f32>` whose lap starts at the threshold
- [x] 3.2 `State::head` reads `light_at`; the painter needs no change
- [x] 3.3 While thinking and before the threshold the loop sleeps to it instead of asking for a 16 ms frame
- [x] 3.4 Tests: no light at 200 ms, the rim's start at 600 ms, and a 300 ms answer that never shows one

## 4. The indicator shows only while the view moves

- [x] 4.1 `thumb_at` is set by a wheel notch, a touchpad slide and a page key, and by nothing else
- [x] 4.2 A pure `thumb_alpha(elapsed)`: 1.0 to 700 ms, linear to 0 at 900 ms
- [x] 4.3 `panel::draw` takes that alpha and paints the same 3 px `LABEL_ACCENT` rectangle through `render::dim`
- [x] 4.4 The loop sleeps through the hold and asks for frames through the fade
- [x] 4.5 Tests: the alpha at 0, 700, 800 and 900 ms, and a streaming transcript nobody scrolled drawing no indicator

## 5. The replies dissolve at the bottom edge

- [x] 5.1 One edge factor per visible row in `draw_replies`, a ramp one row tall, multiplying the ink fade it already passes to `draw_row`
- [x] 5.2 The selection band and the copy cursor take the same factor
- [x] 5.3 Tests: a row in the ramp is dimmer than the same row above it, and rows above the ramp are untouched
- [ ] 5.4 `--preview` frames at 1× and 2× looked at before the live session (1× seen; the dissolve has not been seen on screen, only in its frame test)

## 6. Docs and the gate

- [x] 6.1 README: the light waits half a second, the collapse takes 200 ms, the indicator shows while scrolling, the replies dissolve into the card's edge
- [x] 6.2 `CLAUDE.md` says the 3 px indicator appears while the view moves rather than being always on
- [x] 6.3 `docs/live-checklist.md`: a fast answer shows no light; the indicator appears and fades; the close is quicker than the open; the bottom edge dissolves
- [x] 6.4 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`, `--preview`
