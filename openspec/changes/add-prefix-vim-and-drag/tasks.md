# Tasks

## 1. The cap follows the gap

- [x] 1.1 Replace the fixed `FAR_ROOM` with `REPLY_SHARE × edge_gap`, clamped to the room above the bar, and feed the drag offset into `edge_gap` — verify: a test checks the cap at 1080 and 1440, that it equals the gap at 38 %, and that a gap covering most of the output is clamped so the panel still fits
- [x] 1.2 Recompute the panel size on every drag step — verify: dragging the panel up grows the cap, the replies never re-wrap, and the panel never leaves the output

## 2. Drag by the field

- [x] 2.1 Bind a pointer for the seat and hand its frames to the state — verify: `aside` starts and exits cleanly with a pointer present, and a compositor without one still runs
- [x] 2.2 Start a drag only after the left button has been held `DRAG_HOLD_MS` (250), carry the panel on motion by its margins, and clamp both margins to the output — verify: a short click moves nothing, a press-and-move moves by the pointer's delta, and the loop wakes while a press is pending
- [x] 2.3 Leave nothing behind — verify: nothing is written anywhere on drag, and the next run starts anchored

## 3. Prefix and vim modes

- [x] 3.1 Add `Mode` and make `Ctrl+T` enter the prefix; a one-shot table maps `[` to vim mode and everything else, `Esc` included, to nothing — verify: tests cover the prefix's three cases, including that no consumed key reaches the input line
- [x] 3.2 Map vim keys to row deltas and reuse `move_highlight` — verify: a pure test covers `j`/`k`/`n`/`p`/`h`/`l`/`Ctrl+j`/`Ctrl+k` and that every other key maps to nothing
- [x] 3.3 Keep `Enter` and `Alt+C` working in vim mode, and make `Esc` leave the mode without cancelling — verify: a test asserts `Esc` in vim mode cancels no request and a second `Esc` in typing mode does
- [x] 3.4 Disable the input method on entering vim mode and ignore commits while it is on — verify: an `ime::Action::Commit` event in vim mode leaves the input line unchanged
- [x] 3.5 Draw `-- VIM --` right-aligned in the bar and hide the caret — verify: a panel test finds the label's ink in the bar only in vim mode, with no caret, and the bar geometry unchanged

## 4. Docs

- [x] 4.1 Update the key table in `README.md`, `docs/aside.1`, `docs/live-checklist.md` and `CLAUDE.md` rule 3 for the prefix, vim mode and the drag, and the cap rule in the first three — verify: each document names all three, and none still says the panel cannot be moved or that the cap is `output − 64`

## 5. Integration

- [x] 5.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [x] 5.2 Render `aside --preview out.ppm` and look at it — verify: the bar is unchanged when not in vim mode
- [ ] 5.3 Run a real session — verify: `Ctrl+T [` gives `-- VIM --`, `j`/`k`/`n`/`p` move the highlight and type nothing, `Esc` leaves the mode mid-stream, and a held left button drags the panel
