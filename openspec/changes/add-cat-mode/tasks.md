# The cat mode — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`. Steps carry
> `- [ ]` boxes; tick one as it lands.

**Goal:** A second look for the panel, chosen by `mode = "cat"` in the configuration
file, in which a black cat comes out of a screen edge, takes the question on its
magnified paw, licks the paw while the request runs, and answers in a bubble that comes
out of its mouth.

**Architecture:** The panel's surface, cards, text, keys, scroll, selection and markdown
are untouched. One extra layer surface — the stage — carries the cat, is placed by
margins from the panel's own rect, and lands with its right edge on the input card's
left edge, vertically centred, so the paw, the pad's rim and the bubble's tail are one
geometric identity. `src/cat.rs` holds the geometry, the pose clock and the painter; the
pad and the bubble's growth origin are the panel's own drawing under a mode.

**Tech Stack:** Rust 2024, `smithay-client-toolkit` 0.21, `wayland-client`, `wl_shm` +
`skrifa` + `ab_glyph_rasterizer` software rendering. No new dependency.

**Spec:** `openspec/changes/add-cat-mode/design.md`, `.../specs/chat-panel/spec.md`,
`.../specs/cat-stage/spec.md`.

## Global Constraints

- Zero dependencies beyond the seven in `Cargo.toml`; `pulldown-cmark` stays pinned.
- Nothing on disk: no asset, no cache, no log. The cat is code.
- Single-file modules, pure functions and constants, BGRA colours, ASCII source,
  English comments that say why in one line.
- Software rendering only: `render::rect`, per-row spans, coverage rasterising.
- `mode` accepts `"default"` and `"cat"` and nothing else; a refusal names the file and
  the line and opens no panel.
- No key binding, token value or sandbox change.
- Every non-trivial change carries an assertion-style test in the module it touches.
- Gate: `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.
  Baseline on this machine: 282 pass, and two pre-existing failures that need a CJK font
  this machine does not have (`font::tests::fallback_chain_covers_cjk_for_latin_primary`,
  `panel::tests::a_long_question_scrolls_to_keep_its_caret`). `groff` is not installed,
  so the man-page check cannot run here.
- Version control is jj: `jj describe -m "..."` then `jj new`, imperative subject under
  72 characters.

## Review Focus

1. **A fractional-scale output.** The arm, the pad's rim and the bubble's tail meet at
   one point; at buffer scale 2 or 3 with the viewport scaling down, that point must stay
   one point. A reasonable reader sees a one-pixel step if it does not.
2. **`size` at the ends of its range on a narrow output.** The stage shrinks to the room
   and disappears under 180 logical px. Nothing may panic, and no half-cat may be drawn.
3. **A failed or cancelled request.** The cat must stop licking, the bubble must show the
   error or the note, and the paw must come back — not a cat licking forever.
4. **The pointer over the stage.** Every click and every wheel must pass through the
   stage, including where it overlaps the panel's shadow margin.
5. **A compositor that stacks the two layer surfaces the other way.** The overlap is only
   the 26 px shadow column, so the worst case is a tinted paw — no missing cat, no hole.

---

### Task 1: The mode's configuration key

**Files:**
- Modify: `src/provider.rs` (`parse_config`, its known-key list, its tests)
- Modify: `src/main.rs` (`main`, `Options`, `State`)
- Test: `src/provider.rs` `mod tests`

**Interfaces:**
- Produces: `provider::parse_config` accepts `mode`; `main::Skin { Default, Cat }` with
  `fn skin(entries: &[(String, String)]) -> Skin`; `State::skin: Skin`.
- Consumes: nothing.

- [x] **Step 1: Write the failing test** — in `src/provider.rs`:

```rust
#[test]
fn mode_is_a_known_key_with_two_values() {
    for good in ["default", "cat"] {
        assert!(parse_config(&format!("mode = \"{good}\"")).is_ok(), "{good}");
    }
    let (line, message) = parse_config("mode = \"dog\"").expect_err("must fail");
    assert_eq!(line, 1);
    assert!(message.contains("mode"), "{message}");
}
```

- [x] **Step 2: Run it, watch it fail.** `cargo test mode_is_a_known_key -- --nocapture`
  Expected: FAIL, "unknown key".

- [x] **Step 3: Implement.** In `parse_config`'s `match key`, beside `"animations"`:
`"mode" => match value { "default" | "cat" => {} _ => return Err((line, format!("`mode` must be \"default\" or \"cat\": {value:?}"))) }`. Update the doc comment's key list.

- [x] **Step 4: Read it.** In `src/main.rs`, `Skin { Default, Cat }`, `fn skin(entries) -> Skin`
  (`Cat` only for `mode = "cat"`), stored in `Options.skin` and `State.skin`.

- [x] **Step 5: Test the reading** — in `src/main.rs`'s tests:

```rust
#[test]
fn the_mode_key_picks_the_skin() {
    assert_eq!(skin(&[]), Skin::Default);
    assert_eq!(skin(&[("mode".into(), "cat".into())]), Skin::Cat);
    assert_eq!(skin(&[("mode".into(), "default".into())]), Skin::Default);
}
```

- [x] **Step 6: Run.** `cargo test mode` → PASS. `cargo clippy --all-targets -- -D warnings` → clean.

- [x] **Step 7: Commit.** `jj describe -m "feat: add the cat mode's configuration key"` then `jj new`.

---

### Task 2: Where the stage lands, and where it comes from

**Files:**
- Create: `src/cat.rs` (with `mod cat;` in `src/main.rs`)
- Test: `src/cat.rs` `mod tests`

**Interfaces:**
- Produces:
  - `pub enum Edge { Top, Bottom, Left, Right }`, `pub fn roll_edge(seed: u64) -> Edge`,
    `pub fn seed() -> u64` (clock nanoseconds mixed with the pid).
  - `pub struct Panel { pub x: i32, pub y: i32, pub w: u32, pub h: u32, pub bar_h: u32 }` —
    the panel surface's place on the output in logical px, its size, and the input card's
    height.
  - `pub struct Stage { pub x: i32, pub y: i32, pub size: u32 }` — a square window.
  - `pub const STAGE: f32 = 260.0; pub const STAGE_FLOOR: u32 = 180;`
  - `pub fn stage_size(size: f32, room: u32) -> Option<u32>`
  - `pub fn landing(panel: Panel, size: f32) -> Option<Stage>`
  - `pub fn peek(edge: Edge, output: (u32, u32), panel: Panel, size: f32) -> Option<Stage>`
  - `pub fn hop(from: Stage, to: Stage, t: f32) -> Stage`
- Consumes: `render::SHADOW`, `render::SEPARATION`.

- [x] **Step 1: Write the failing tests** — the landing identity and the four edges:

```rust
#[test]
fn the_stage_lands_with_its_right_edge_on_the_input_card() {
    let panel = Panel { x: 614, y: 400, w: 692, h: 300, bar_h: 30 };
    let stage = landing(panel, 1.0).expect("room to the left");
    assert_eq!(stage.size, 260);
    assert_eq!(stage.x + stage.size as i32, panel.x + render::SHADOW as i32); // card's left edge
    let bar_centre = panel.y + panel.h as i32 - render::SHADOW as i32 - panel.bar_h as i32 / 2;
    assert_eq!(stage.y + stage.size as i32 / 2, bar_centre);
}

#[test]
fn no_room_means_no_cat() {
    // 160 px to the left of the card is under the 180 floor.
    let panel = Panel { x: 134, y: 400, w: 692, h: 300, bar_h: 30 };
    assert_eq!(landing(panel, 2.0), None);
}

#[test]
fn the_stage_shrinks_to_the_room_before_it_disappears() {
    assert_eq!(stage_size(2.0, 320), Some(320));
    assert_eq!(stage_size(2.0, 179), None);
}

#[test]
fn every_edge_is_rolled_and_each_peek_is_flush_with_its_edge() {
    let seen: std::collections::BTreeSet<_> = (0..64).map(roll_edge).collect();
    assert_eq!(seen.len(), 4, "{seen:?}");
    let panel = Panel { x: 614, y: 400, w: 692, h: 300, bar_h: 30 };
    let out = (1920, 1080);
    // Centred on the input card's axis: x 614 + 346 − 130, y 659 − 130.
    for (edge, want) in [
        (Edge::Bottom, (830, 1080 - 260)),
        (Edge::Top, (830, 0)),
        (Edge::Left, (0, 659 - 130)),
        (Edge::Right, (1920 - 260, 659 - 130)),
    ] {
        let stage = peek(edge, out, panel, 1.0).expect("room");
        assert_eq!((stage.x, stage.y), want, "{edge:?}");
    }
}
```

- [x] **Step 2: Run them, watch them fail.** `cargo test cat::` → FAIL, no such module.

- [x] **Step 3: Implement** `src/cat.rs` with those items. `landing` computes
  `room = panel.x + render::SHADOW as i32` (the output's left edge to the card's left
  edge), `size = stage_size(size, room as u32)?`, `x = panel.x + SHADOW - size`,
  `y = bar_centre - size / 2`. `peek` centres the square on the input card's axis and
  puts it flush with its edge. `hop` lerps `x`/`y` and lifts the arc by
  `0.35 * size * sin(pi * t)`, so `hop(a, b, 0) == a` and `hop(a, b, 1) == b`. `roll_edge`
  folds the seed and takes it modulo four. `seed()` mixes `SystemTime` nanoseconds with
  `std::process::id()` — no dependency.

- [x] **Step 4: Run.** `cargo test cat::` → PASS.

- [x] **Step 5: Commit.** `jj describe -m "feat: place the cat's stage beside the pad"` then `jj new`.

---

### Task 3: The pose clock

**Files:**
- Modify: `src/cat.rs`
- Test: `src/cat.rs` `mod tests`

**Interfaces:**
- Produces: `pub enum Mood { Idle, Thinking, Answering }`,
  `pub enum Pose { Peek, Hop, Arm, Waiting, Lick, Speak, Extend }`,
  `pub fn pose(entrance: Duration, mood: Mood, since_mood: Duration, since_finish: Option<Duration>, animate: bool) -> Pose`,
  and the constants `PEEK_MS 420`, `HOP_MS 400`, `ARM_MS 300`, `ENTER_MS 1120`,
  `LICK_AFTER_MS 300`, `SPEAK_MS 200`, `EXTEND_MS 260`.
- Consumes: nothing from Task 2 but the file.

- [x] **Step 1: Write the failing tests:**

```rust
#[test]
fn the_entrance_runs_in_order_and_stops_when_the_question_goes() {
    let ms = |m| Duration::from_millis(m);
    assert_eq!(pose(ms(0), Mood::Idle, ms(0), None, true), Pose::Peek);
    assert_eq!(pose(ms(500), Mood::Idle, ms(0), None, true), Pose::Hop);
    assert_eq!(pose(ms(900), Mood::Idle, ms(0), None, true), Pose::Arm);
    assert_eq!(pose(ms(1200), Mood::Idle, ms(0), None, true), Pose::Waiting);
    // Sent mid-entrance: the cat is where it lands, at once.
    assert_eq!(pose(ms(500), Mood::Thinking, ms(0), None, true), Pose::Waiting);
}

#[test]
fn the_lick_belongs_to_thinking_and_stops_at_the_first_text() {
    let ms = |m| Duration::from_millis(m);
    assert_eq!(pose(ms(2000), Mood::Thinking, ms(100), None, true), Pose::Waiting);
    assert_eq!(pose(ms(2000), Mood::Thinking, ms(400), None, true), Pose::Lick);
    assert_eq!(pose(ms(2000), Mood::Answering, ms(0), None, true), Pose::Speak);
    assert_eq!(pose(ms(2000), Mood::Answering, ms(400), None, true), Pose::Waiting);
}

#[test]
fn the_paw_comes_back_after_the_answer() {
    let ms = |m| Duration::from_millis(m);
    assert_eq!(pose(ms(2000), Mood::Answering, ms(400), Some(ms(100)), true), Pose::Extend);
    assert_eq!(pose(ms(2000), Mood::Answering, ms(400), Some(ms(400)), true), Pose::Waiting);
}

#[test]
fn animation_off_is_the_landed_cat_in_the_first_frame() {
    assert_eq!(pose(Duration::ZERO, Mood::Idle, Duration::ZERO, None, false), Pose::Waiting);
}
```

- [x] **Step 2: Run them, watch them fail.**

- [x] **Step 3: Implement** `pose`: `!animate` → `Waiting`; `Mood::Idle` → the entrance
  timeline; `Thinking` → `Lick` once `since_mood >= LICK_AFTER_MS`; `Answering` →
  `Extend` while `since_finish < EXTEND_MS`, else `Speak` while `since_mood < SPEAK_MS`,
  else `Waiting`.

- [x] **Step 4: Run.** `cargo test cat::` → PASS.

- [x] **Step 5: Commit.** `jj describe -m "feat: give the cat its poses"` then `jj new`.

---

### Task 4: The cat, drawn

**Files:**
- Modify: `src/cat.rs` (`draw`, the inks, the shape helpers)
- Modify: `src/main.rs` (`preview`: a `cat.<pose>.ppm` per pose)
- Test: `src/cat.rs` `mod tests`

**Interfaces:**
- Produces: `pub fn draw(buf: &mut [u8], w: u32, h: u32, s: u32, bar_h: u32, pose: Pose, since: Duration, bubble: bool)`
  where `w`/`h` are the stage's buffer size, `s` the buffer scale and `bar_h` the input
  card's height in logical px.
- Consumes: `render::rect`, `render::bgra`, `render::FG_NORMAL`, `render::SEPARATION`.

- [x] **Step 1: Write the failing tests** — the three contacts and the window:

```rust
#[test]
fn the_paw_and_the_bubble_tail_land_on_the_contacts() {
    let (w, h, bar_h) = (260u32, 260u32, 30u32);
    // The identity holds at every buffer scale: the seam is a point, not a range.
    for s in [1u32, 2, 3] {
        assert_eq!(paw(w * s, h * s), (w * s, h * s / 2)); // the pad's left rim, centred
        let corner = tail_start(w * s, h * s, s, bar_h);
        assert_eq!(corner.0, w * s);
        assert!(corner.1 > 0 && corner.1 < h * s / 2, "{corner:?} at s {s}"); // inside the stage
    }
}

#[test]
fn the_cat_never_paints_outside_its_window() {
    let (w, h) = (260u32, 260u32);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    draw(&mut buf, w, h, 1, 30, Pose::Waiting, Duration::ZERO, false);
    // The corners of the square are outside any pose's reach.
    for (x, y) in [(0, 0), (w - 1, 0), (0, h - 1)] {
        let px = &buf[((y * w + x) * 4) as usize..][..4];
        assert_eq!(px[3], 0, "transparent at {x},{y}");
    }
}
```

- [x] **Step 2: Run them, watch them fail.**

- [x] **Step 3: Implement** the painter: `paw(w, h) -> (u32, u32)` and
  `tail_start(w, h, s, bar_h) -> (u32, u32)` as pure functions (the tests pin them); the
  silhouette — body, head, two ears, two eyes, tail — as filled circles and per-row
  spans in `#101014`, with a rim light in `render::FG_NORMAL` along the upper-left edges;
  the arm from the shoulder to `paw`; the tongue over the paw in `Lick`; the mouth wedge
  in `Speak`; the paw's reach in `Extend`; and, when `bubble`, the tail as a filled
  triangle from `tail_start` toward the mouth.

- [x] **Step 4: Look at it.** In `preview`, write `cat.<pose>.ppm` for `Peek`, `Hop`,
  `Arm`, `Waiting`, `Lick`, `Speak`. Run
  `cargo run -- --preview /tmp/aside.ppm` and open the six files; the cat must read as a
  cat, and each pose must differ.

- [x] **Step 5: Run.** `cargo test cat::` → PASS.

- [x] **Step 6: Commit.** `jj describe -m "feat: draw the cat"` then `jj new`.

---

### Task 5: The stage's surface

**Files:**
- Modify: `src/main.rs` (`State`, the frame pass, the output and configure handlers, the size/margin handlers)
- Test: `src/main.rs` `mod tests`

**Interfaces:**
- Consumes: `cat::{Stage, Panel, landing, peek, hop, pose, draw, roll_edge, seed, Mood}`.
- Produces: `State::stage: Option<cat::Stage>`, `State::stage_surface: Option<LayerSurface>`,
  `State::stage_buffer: Option<Buffer>`, `State::entrance: Instant`, and
  `fn stage_panel(&self) -> cat::Panel` (the panel surface's place and its bar's height).
- `fn mood_of(phase: Phase) -> cat::Mood` and `fn stage_pose(&self, now) -> cat::Pose`,
  pure enough to test.

- [x] **Step 1: Write the failing test** — the phase mapping:

```rust
#[test]
fn the_cat_answers_what_the_panel_is_doing() {
    assert_eq!(mood_of(Phase::Idle), cat::Mood::Idle);
    assert_eq!(mood_of(Phase::Thinking), cat::Mood::Thinking);
    assert_eq!(mood_of(Phase::Answering), cat::Mood::Answering);
}
```

- [x] **Step 2: Run it, watch it fail.**

- [x] **Step 3: Implement** the stage's lifecycle: create it in cat mode once the panel's
  output is known (`--output`, or the panel's `wl_surface.enter`), `Layer::Overlay`, the
  same namespace, `KeyboardInteractivity::None`, an empty input region, the panel's buffer
  scale and viewporter; wait for its own configure before the first buffer; anchor
  `TOP | LEFT` and set `top`/`left` margins from `cat::Stage`; allocate its buffer from
  the panel's `SlotPool`; paint it after the panel in the same frame pass, damaged whole
  and committed after; recompute the rect in `apply_size`/`set_size`/`toggle` and on the
  drag handlers; start the entrance clock at its first commit.

- [x] **Step 4: Run it by hand.** With `mode = "cat"`, summon: the cat peeks from an edge,
  hops, and its paw sits on the input card's left rim. Move the panel with `Alt+hjkl` and
  resize it with `Ctrl+Alt+-`/`+`: the stage follows. Click where the cat is: the click
  passes through. If `groff` is unavailable, skip the man check.

- [x] **Step 5: Run.** `cargo test` → the same 282 pass and the two known environment
  failures; `cargo clippy --all-targets -- -D warnings` → clean.

- [x] **Step 6: Commit.** `jj describe -m "feat: show the cat on its own surface"` then `jj new`.

---

### Task 6: The input is the paw

**Files:**
- Modify: `src/panel.rs` (`draw`, `draw_bar`, `Look`)
- Test: `src/panel.rs` `mod tests`

**Interfaces:**
- Produces: `Look.skin: Skin` and `fn draw_pad(...)` used by `draw_bar` in the cat mode.
- Consumes: `main::Skin` from Task 1.

- [x] **Step 1: Write the failing test** — the pad keeps the bar's box:

```rust
#[test]
fn the_pad_keeps_the_bars_box_and_its_text() {
    // The same frame painted as a bar and as a pad: the text's ink is in the same
    // columns and rows, so only the card's own drawing differs.
    let bar = paint_with(Skin::Default);
    let pad = paint_with(Skin::Cat);
    for x in [PAD + 1, PAD + 40, w / 2] {
        assert_eq!(glyph_rows(&bar, x), glyph_rows(&pad, x), "column {x}");
    }
}
```

- [x] **Step 2: Run it, watch it fail.**

- [x] **Step 3: Implement** the pad: the same rounded box, drawn with the palm's shape and
  four toe bumps inside the rim at its two ends, the rim lit on the side the cat is on.
  The text, caret, marquee, hint and scroll paths are the bar's, untouched.

- [x] **Step 4: Run.** `cargo test panel::` → PASS.

- [x] **Step 5: Commit.** `jj describe -m "feat: draw the input as the cat's paw"` then `jj new`.

---

### Task 7: The bubble

**Files:**
- Modify: `src/panel.rs` (`Look`, `draw`, the growth)
- Modify: `src/transcript.rs` (`current_start`)
- Modify: `src/main.rs` (`state_of`, the first-text and finish transitions)
- Test: `src/panel.rs`, `src/transcript.rs`, `src/main.rs` `mod tests`

**Interfaces:**
- Produces: `Look.mouth: Option<(f32, f32)>` — the growth's origin in surface coordinates;
  `Transcript::current_start` returns the previous turn's start while the current turn has
  no drawable entry; `fn state_of_skin(phase: Phase, skin: Skin, previous: bool) -> panel::State`,
  with the existing `state_of` delegating to it for `Skin::Default`.
- Consumes: Task 5's `stage_panel`.

- [x] **Step 1: Write the failing tests:**

```rust
// transcript.rs: a turn with nothing drawn yet shows the previous turn.
#[test]
fn an_empty_turn_shows_the_last_one() {
    let mut t = Transcript::default();
    t.ask("first", &font); t.delta("an answer", &font, w); t.finish_answer(&font);
    let before = t.rows(blocks).len();
    t.ask("second", &font);
    assert_eq!(t.rows(blocks).len(), before, "the last answer stays");
    t.delta("new text", &font, w);
    assert_eq!(t.rows(blocks).len(), 1, "and the new one replaces it");
}
```

```rust
// main.rs: the bubble stays while the next request runs, and a failure is an answer too.
#[test]
fn the_cat_keeps_the_bubble_while_it_thinks() {
    assert_eq!(state_of_skin(Phase::Thinking, Skin::Cat, true), panel::State::Replies);
    assert_eq!(state_of_skin(Phase::Thinking, Skin::Cat, false), panel::State::Bar);
    assert_eq!(state_of_skin(Phase::Thinking, Skin::Default, true), panel::State::Bar);
    // A failed or cancelled request lands in Answering: the lick stops and the paw
    // comes back, which is what a reader who was refused needs to see.
    assert_eq!(state_of_skin(Phase::Answering, Skin::Cat, true), panel::State::Replies);
}
```

```rust
// panel.rs: growth from the mouth clips toward the mouth's corner first.
#[test]
fn the_bubble_grows_out_of_the_mouth() {
    // At the start of the growth the lit pixels are the ones nearest the mouth.
    let lit = lit_bounds(&paint_growing(0.05, Some((40.0, 300.0))));
    assert!(lit.top > h / 2, "nothing above the bar yet: {lit:?}");
}
```

- [x] **Step 2: Run them, watch them fail.**

- [x] **Step 3: Implement.** `Transcript::current_start`: walk back past a current turn
  that has no drawn entry. `Look.mouth` + the clipped growth in `panel::draw`. `state_of`
  gains the skin and the "a previous turn is drawn" fact. On the first text of a request
  the bubble is already there, so its growth does not run again; on `finish_answer` the
  stage's `since_finish` starts, which is what makes the paw come back.

- [x] **Step 4: Run.** `cargo test` → the known baseline plus PASS for the new tests.

- [x] **Step 5: By hand.** With `mode = "cat"`: ask, watch the lick, watch the mouth and
  the bubble, then ask a second question — the old answer stays in the bubble until the
  new text arrives.

- [x] **Step 6: Commit.** `jj describe -m "feat: let the cat answer in a bubble"` then `jj new`.

---

### Task 8: The docs, the project's rule, and the gate

**Files:**
- Modify: `README.md` (the configuration section, the file table)
- Modify: `docs/aside.1` (the key)
- Modify: `CLAUDE.md` (rule 4, scoped to the default mode)
- Modify: `openspec/changes/add-cat-mode/tasks.md` (tick what landed)

- [x] **Step 1: README.** `mode` in the key list, `mode = "cat"` in the example, and one
  sentence on what the cat mode draws.
- [x] **Step 2: The man page.** The same, in the man page's voice.
- [x] **Step 3: `CLAUDE.md`.** Rule 4 becomes the default mode's chrome, and one sentence
  names the cat mode's: the stage, the pad, the bubble.
- [x] **Step 4: The gate.** `cargo test` (282 pass plus the two known environment
  failures), `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.
- [x] **Step 5: Commit.** `jj describe -m "docs: describe the cat mode"` then `jj new`.
