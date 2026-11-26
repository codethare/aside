# Tasks

## 1. Reading and feedback

- [x] 1.1 Streaming and the end of a request move the view only when it is already at the tail; a test pins that a scrolled-up view keeps its top and reports new text
- [x] 1.2 A `new content` pill is drawn at the replies' bottom-left while that flag is set, and scrolling back to the tail clears it
- [x] 1.3 Copy, a cut paste and a returned question write one toast in the bar; a paste at the cap says it was cut
- [x] 1.4 A failed request glows the bar's inner rim for 700 ms and places the caret at the end of the restored question

## 2. Focus and waiting

- [x] 2.1 An unfocused bar draws a faint inner rim, driven by the keyboard-held state; a focused frame draws none
- [x] 2.2 While a request runs the bar's right side reads `Esc cancel`, and after three seconds adds the elapsed seconds
- [x] 2.3 Typing during the particle leaving clears the question in the frame the text appears
- [x] 2.4 A follow-up draws the previous answer's card back up into its top border over 300 ms, accelerating like a suction, starts the request after the retract, and leaves no empty card; typing cancels the pending send
- [x] 2.5 The reply card drips down from its top border, which stays attached, with a small spring overshoot; closing draws it back up into the top border, with no second bar below

## 3. Startup and discovery

- [x] 3.1 `--font-size` and `--font-family` override the configuration for the run; an out-of-range size is refused with a line on stderr
- [x] 3.2 `--preview` uses the overridden size, so the particle density can be compared without a restart
- [x] 3.3 `--help` lists the keys, including the `Ctrl+T` prefix and copy mode

## 4. Contrast and docs

- [x] 4.1 Leaving particles use full white, brighter than the prompt ink
- [x] 4.2 README, the man page and the live checklist describe the new cues and flags
- [ ] 4.3 A live check: scroll up during an answer, confirm the view stays and the pill appears; focus another window and confirm the rim; paste over the cap and read the toast; fail a request and see the glow

## 5. Gate

- [x] 5.1 `cargo test` (only the pre-existing font-environment failures), `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
