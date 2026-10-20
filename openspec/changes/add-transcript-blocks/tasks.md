# Tasks

## 1. Block geometry in the transcript

- [x] 1.1 Give `Entry` a cached `y` and `h` spanning its rows plus card padding, and maintain the chain in `push`, `delta`, `finish`, `cancel`, `fail` and `rewrap` — verify: an assertion test checks that `total_height()` equals the sum of block heights plus gaps, that consecutive blocks do not overlap or leave a hole, and that growing the tail entry moves no earlier block
- [x] 1.2 Expose `total_height()` and `row_y(index)` and give `Row` its block context, so the painter knows where a line goes and whether it opens or closes a block — verify: an assertion test walks every row and reports a `row_y` inside its own block
- [x] 1.3 Delete the unused `Row::first` and its assertion — verify: `cargo clippy -- -D warnings` and the test that asserted it are gone, with no other behaviour changed
- [x] 1.4 An entry with no text occupies no height and draws no gap — verify: a test asks a question, opens an empty answer and checks `total_height()` equals the question's block alone

## 2. The viewport in pixels

- [x] 2.1 Move `Viewport` to pixel `top`/`height` with `ensure_visible(y, row_h)`, keep `MAX_ROWS` as a pixel cap of `MAX_ROWS * row_h`, and make `Up`/`Down` step one `row_h` — verify: the viewport tests assert one-line steps, the cap, and that a scrolled-up reader does not move when the tail grows
- [x] 2.2 Point `main.rs` at the new viewport API wherever it sets `height`, `clamp`s or follows the tail — verify: `cargo test` passes with the sizing still resolving to the same heights as before for a transcript with no cards

## 3. The card

- [x] 3.1 Add a rounded-rectangle fill to `render` on top of `rounded_span`/`rounded_inset`, with the corners antialiased exactly as the panel's edge — verify: a test paints a card on a known buffer and checks the fill colour in the middle, partial coverage at the corner, and clear pixels outside
- [x] 3.2 Add the `BG_CARD` token and put `FG_NORMAL` on `BG_CARD` into the WCAG pair test — verify: the contrast assertion passes at 4.5:1 or better
- [x] 3.3 Paint blocks instead of rows in `panel::draw`: a card for answer, error and note entries (inset `PAD`, radius 12, 8 px padding, text inset 8 px further), a plain row for a question, an 8 px gap between blocks — verify: a test asserts the card's fill at the answer's first line and the body tone beside the question's text, and that the question's row starts at `PAD`
- [x] 3.4 Wrap the transcript at the card's inner width — verify: the existing wrap and re-wrap tests still pass and no row measures wider than `panel_width - 2 * PAD - 2 * CARD_PAD`
- [x] 3.5 Update `README.md`'s "What it looks like", `docs/aside.1` and `CLAUDE.md` rule 4 to say the frame and bar are `wlmenu`'s and the transcript is a stack of blocks with cards for replies — verify: each of the three documents says it, and none still claims the transcript copies `wlmenu`'s list rows

## 4. The rail

- [x] 4.1 Replace the full-width selection band with a 2 px `BG_SEL` rail at the highlighted line's block inset, drawn above the card fill and below the text — verify: a new test asserts the rail's pixels at the line's left inset, that no pixel of the row is `BG_SEL` beyond the rail, and that moving the highlight changes nothing else in the frame
- [x] 4.2 Keep `Alt+C` copying exactly the highlighted line from inside a card — verify: the clipboard test covers a line inside a card
- [x] 4.3 Delete the old band test rather than re-asserting it — verify: `the_selection_is_clipped_inside_the_border` is gone, and the commit message names the behaviour change

## 5. Panel height

- [x] 5.1 Derive the answering height from `transcript.total_height()` capped at `MAX_ROWS * row_h`, replacing `rows * bar` in `size_for` — verify: a test asserts the panel grows by a card's padding when the first answer arrives, and that a long transcript stops at the cap
- [x] 5.2 Re-check the bar, divider and thinking ball are untouched — verify: their existing tests pass unchanged

## 6. Integration

- [x] 6.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean
- [x] 6.2 Render `aside --preview out.ppm` and look at it — verify: a question, a carded answer, a carded failure and a cancelled note are distinguishable, the rail marks one line, and nothing overflows
- [ ] 6.3 Run a real session under sway or river — verify: the panel grows per block while streaming, `Up`/`Down`/`PageUp`/`PageDown` scroll by line, `Alt+C` copies the highlighted line, and `Esc` still cancels then closes
