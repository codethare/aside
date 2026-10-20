# Tasks

## 1. The replies and the bar

- [x] 1.1 Stop laying out a question: `ask` keeps its text for `request_turns` and leaves `Lines` empty, and `rewrap` skips it — verify: a test asserts that a question and its answer produce answer rows only, that the question still reaches `request_turns`, and that a question adds no height
- [x] 1.2 Assert the highlight and the copy key cannot land on a sent question — verify: a test walks every row of a two-turn transcript and finds no `Kind::Question`, and `total_rows` counts reply lines alone

## 2. The panel's arrangement

- [x] 2.1 Paint the bar at the panel's bottom edge with the divider as its top hairline, and the transcript above it clipped to that area — verify: a test asserts the prompt band is the panel's last row, the transcript's first row starts at the panel's top edge, and a card cannot paint over the bar
- [x] 2.2 Move the scroll indicator's track over the transcript area only — verify: the overflow test finds the thumb above the bar and never in it
- [x] 2.3 Derive the visible transcript height from the area above the bar — verify: `transcript_px` still equals the panel's content height for the same number of rows

## 3. The anchor

- [x] 3.1 Anchor the layer surface to the bottom with a bottom margin, keeping the panel's visible gap at 32 px — verify: `aside` appears at the bottom of the output, and the idle surface is still `688 × 78`
- [x] 3.2 Place the input method's caret rectangle on the bar's new row — verify: the value is derived from the panel height rather than assumed to be the top row

## 4. Docs

- [x] 4.1 Update `README.md`, `docs/aside.1`, `docs/live-checklist.md` and `CLAUDE.md` rule 4 for the bottom anchor, the bar below the replies and the replies-only transcript — verify: each document says all three, and none still says the bar is the top row or that questions are drawn

## 5. Integration

- [x] 5.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [x] 5.2 Render `aside --preview out.ppm` and look at it — verify: the bar is at the bottom, the replies above it, no question row anywhere
- [ ] 5.3 Run a real session — verify: the panel sits at the bottom of the output, the bar does not move while an answer streams, and `Up`/`Down`/`Alt+C` address reply lines
