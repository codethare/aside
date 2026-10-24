# Live checklist

Everything here needs a real Wayland session and a real keybinding, so it is the
one part of the test plan that cannot run headless. Each line is a step with a
result to report back; anything that fails is worth reporting as it happened,
not reworded.

## Before starting

```sh
export COMMAND_CODE_API_KEY=...
bind=SUPER,A   # substitute your compositor's binding syntax
```

## The panel

1. Press the keybinding. The panel appears in the lower half of the output,
   centred, its field just below the vertical centre. Move the pointer onto it (or
   click it) and type: the characters land in the input line.
1b. While the panel is up, switch to another window with your compositor's binding
   and type there: the characters land in that window, not in the panel. Go back to
   the panel, type again: they land in the panel. This is the check that the panel
   does not hold the keyboard; if a keybinding alone does not move focus, one click
   on the window is the compositor's requirement, not the panel's.
2. Type `what does EAGAIN mean` and press `Enter`. The bar does not move or change
   size: a light starts running round its rim and laps it about every 1.4 s. When
   the first text arrives the bar grows into the panel and only then does the reply
   card rise out of it. Opening the panel is a line spreading out of a point and
   unfolding up, not a snap, and the input line is revealed as the bar reaches it
   rather than appearing on the line before it.
3. Ask a follow-up. The panel does not change size at all: the light starts on its
   bar's rim and the box stays where it was. The box then holds the new answer alone
   — the earlier one is not on screen — and the answer still takes the earlier
   exchange into account.
4. `Up`, `Down`, `PageUp`, `PageDown` move the highlighted line and the view
   follows it; the highlighted line is brighter than its neighbours and starts
   2 px further right, and nothing is drawn in the left margin.
5. `Alt+C` or `Ctrl+C` copies exactly the highlighted line. `Ctrl+V` pastes the
   clipboard into the input line, at the caret, and a missing `wl-paste` shows one
   line in the transcript instead of doing nothing. Paste it somewhere and check
   there is no extra whitespace or transcript around it.
5b. `Ctrl+T`: the bar changes tone, takes no input and reads `press [ to enter copy
   mode`, with no caret; any other key leaves it and types nothing, and the bar
   returns exactly as it was. Then `Ctrl+T` then `[`: the bar reads `-- COPY --`,
   the caret is gone, and a cursor sits in the replies.
5b2. In copy mode `h`/`l` move a character, `j`/`k` a line, `w`/`b` a word,
   `Ctrl+d`/`Ctrl+u` half a page, `g`/`G` to the ends. `Space` selects and `Enter`
   copies and leaves, with `copied N lines` in the bar; `q` leaves without copying.
   No letter reaches the input line.
5b3. `Alt+hjkl` moves the panel in every mode and `Alt+Ctrl+hjkl` ten times as far;
   it stops at the screen edges.
5c. Press the left button on the bar and move at once: the panel follows from the
   first motion, in both directions, stays on screen, and does not change size
   while it moves. Keep dragging after it has started moving — a slow sweep, then a
   fast one, then drag past the screen edge and back — and it follows every time: the
   panel moving out from under the pointer is not the pointer letting go. A click
   without moving does nothing, and a new run starts where the anchor puts it.
5c1. Type `hello world`, press `Left` five times: the caret walks back through the
   text and `Backspace` deletes the character it just passed. `Home`/`End` jump to
   the ends, `Ctrl+Left`/`Ctrl+Right` by word, and the pre-edit of a Chinese input
   method appears at the caret, underlined.
5c1b. Type a question past the width of the bar: the line scrolls under the caret,
   the caret stays inside the field, and `Home` brings the first characters back.
   Submit it: the whole question reaches the provider.
5c2. Move the pointer across the replies without holding a button: nothing is
   selected, and a selection made a moment ago stays exactly as it was.
5c3. Press on the first line of an answer and drag to its third line: those three
   lines are drawn highlighted, no text moves, and `Ctrl+C` puts exactly those
   three lines on the clipboard. `Esc` clears the selection and leaves the panel
   open; a second `Esc` closes it.
5d0. Put the pointer over the replies and turn the wheel (and slide two fingers):
   the transcript scrolls, three lines a notch, and it stops at both ends. Over the
   bar, nothing scrolls. Mid-scroll, no two lines ever overlap: a line that is
   partly scrolled out is not drawn at all.
5d. Ask something long enough to overflow the box: the panel is the same size it
   was, the newest lines are the ones on screen, and scrolling reaches the older
   ones.
6. While an answer is streaming, run `pgrep -a bwrap` in another terminal: the
   sandboxed process is there. Press `Esc`: the stream stops, `cancelled` appears,
   the panel stays open, and `pgrep -a bwrap` now finds nothing.
7. Press `Esc` again: the panel closes. Repeat the keybinding while it is open:
   the *same* window comes forward, and no second window appears.
7b. With the panel open, click a terminal behind it and type: the text goes to the
   terminal, not to the panel. Click the panel and type: it goes to the panel.
7c. Put `font_size = "28"` and `font_family = "DejaVu Sans Mono"` in the config and
   start again: the text is that size and that face, the bar is taller, and Chinese
   still renders. A `font_size = "2"` refuses to start, naming the file and line.

## Typing, including an input method

8. With no input method running, type ASCII: characters appear as typed.
9. Start fcitx (or ibus), select a Chinese layout, and compose a question such as
   `什么是 EAGAIN`. The pre-edit is visible while composing, and the committed
   characters arrive whole in the input line.
10. Edit mid-line: `Left`, `Right`, `Home`, `End`, `Backspace`, `Delete`, and
    `Ctrl+Left`/`Ctrl+Right`. The caret moves and the text follows.
11. Paste a multi-line question with the compositor's paste key. It arrives, and
    it wraps instead of being clipped.

## Display

12. On a 1× output, then a 1.5× and a 2× output: text is the same logical size on
    each; above 1× there are no colour fringes on glyph edges.
13. Resize the window while an answer is on screen: the text re-wraps to the new
    width, and dragging it narrow or wide does not leave stray clipped glyphs.
14. Time the first frame on a cold start (`time aside` with no transcript): it
    should be well under 50 ms after the compositor has the surface.
15. Put a `wlmenu` frame next to an `aside --preview out.ppm` frame and check the
    shared tokens against each other: radius, padding, the 16 px card separation,
    row height, the glass tone, the 26 px shadow falloff, the rim's lit top edge, its
    dark band inside, and the corner antialiasing. This is the side-by-side check the design asks for, and
    it needs a real `wlmenu` frame from your session.
14. Scroll back while an answer is streaming: the view stays where it was put.

## Failure paths (optional, each costs one request)

16. Ask a question with `ASIDE_MODEL=nonsense/model`: the transcript shows a
    provider error naming the status, and the panel stays usable.
17. Unset the key and run `aside` from a terminal: one line on standard error
    naming `COMMAND_CODE_API_KEY`, non-zero exit, and no window.
