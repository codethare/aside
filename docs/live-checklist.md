# Live checklist

Everything here needs a real Wayland session and a real keybinding, so it is the
one part of the test plan that cannot run headless. Each line is a step with a
result to report back; anything that fails is worth reporting as it happened,
not reworded.

## Before starting

```sh
export OPENCODE_API_KEY=...
bind=SUPER,A   # substitute your compositor's binding syntax
```

## The panel

1. Press the keybinding. The panel appears in the lower half of the output,
   centred, its field just below the vertical centre. Type without touching the
   pointer: the characters land in the input line. The keyboard arrives because the
   compositor focuses an interactive layer surface when it maps — if typing needs a
   click first, that is the thing to report, with the compositor and its version.
1a. Summon it again while it has the keyboard: it closes, the same way `Esc` closes
   it, and the second launch exits zero. Summon it a third time while that closing is
   still running: the closing is not restarted or stretched.
1a1b. Focus another window (or wait for `tailrace` to move focus), then look at the
   panel: it is still on screen and its caret is gone — a panel without the keyboard
   does not look like it has one. Press the summoning key: it takes the keyboard back
   and the caret comes back with it. If the remap loses the race with the window
   manager's focus request, the panel stays unfocused; press the summoning key again
   and it closes instead of remapping for ever.
1a2. Type a question, focus another window (a binding, or a click on the window), then
   summon the panel again: it stays on screen with the question still in the input
   line, it blinks for one frame as the surface remaps, and typing lands in it. On
   `river` a pointer that only hovers over the panel does not focus it — a click does —
   and no keybinding of the compositor's can.
1a3. Ask a question wider than the bar and watch it while the model thinks: the
   question stays in the bar, scrolling slowly out to its last character, holding,
   coming back, and turning round again while the request is still unfinished. There
   is no caret while it is there. When the answer arrives the question is drawn out
   of the bar — it keeps going the way it was going, fast and fading, out of the edge
   it was heading for — and the bar is an empty line again. Type while the answer
   streams and the question goes at once, the typing being what the line shows.
1b. While the panel is up, switch to another window with your compositor's binding
   and type there: the characters land in that window, not in the panel. Go back to
   the panel, type again: they land in the panel. This is the check that the panel
   does not hold the keyboard; if a keybinding alone does not move focus, one click
   on the window is the compositor's requirement, not the panel's.
2. Type `what does EAGAIN mean` and press `Enter`. The bar does not move or change
   size. Nothing on it lights up for the first half second: the light appears only if
   the request is slower than that, and then it laps the rim about every 1.4 s. When
   the first text arrives the bar grows into the panel and only then does the reply
   card rise out of it. Opening the panel is a line spreading out of a point and
   unfolding up, not a snap, and the input line is revealed as the bar reaches it
   rather than appearing on the line before it.
2b. Ask something the provider answers almost at once (a bare `hi`): the reply
   arrives with no light on the rim at any point — not a flash of one.
2c. Ask something that comes back as markdown: "show me a C loop that retries on
   EAGAIN, with a list and a heading". The code block is monospace on a tinted
   band, its lines stay aligned and a long line wraps inside the block; the list
   has bullets and a wrapped item hangs under its own text; a heading is bold with
   an empty row above it; `**bold**` is brighter and `` `code` `` is monospace, and
   neither shows its asterisks or backticks. Ask for a table: the header is one row
   with a hairline of dashes under it, the columns start in the same place on every
   row, `:---:` and `---:` line their columns up, a long cell wraps and its row grows,
   and no pipe from the source is drawn. Copy a line out of the code block: the
   clipboard holds the code, not the fences.
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
5b. `Ctrl+T`: the bar changes tone, takes no input and reads `press [ for copy
   mode, + / - for text size`, with no caret; any other key leaves it and types
   nothing, and the bar returns exactly as it was. `Ctrl+T` then `+` grows the text
   two pixels, `Ctrl+T` then `-` shrinks it, the bar and the rows follow, and the
   answer on screen re-wraps in the new size; ten presses up stop at 72 and ten down
   stop at 8 without an error. Then `Ctrl+T` then `[`: the bar reads `-- COPY --`,
   the caret is gone, and a cursor sits in the replies.
5b2. In copy mode `h`/`l` move a character, `j`/`k` a line, `w`/`b` a word,
   `Ctrl+d`/`Ctrl+u` half a page, `g`/`G` to the ends. `Space` selects and `Enter`
   copies and leaves, with `copied N lines` in the bar; `q` leaves without copying.
   No letter reaches the input line.
5b3b. `Ctrl+Alt++` makes the panel's box bigger — wider, and with room for more rows —
   15 % each press; the text does not change size, more of it fits, and the lines wrap
   at the new width. `Ctrl+Alt+-` shrinks it, `Ctrl+Alt+BackSpace` returns to the
   default size. The same in typing, prefix and copy mode, and no character reaches
   the input line. `Ctrl` + the wheel resizes by 5 % a notch however fast it is spun -
   it never accelerates - and without `Ctrl` the
   wheel still scrolls. A two-finger pinch resizes with the fingers. It stops at 200 %
   and at 75 %, and where the output cannot hold the panel; restart the panel and it
   is back at its default size.
5b4. `Alt+hjkl` moves the panel in every mode and `Alt+Ctrl+hjkl` ten times as far;
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
   the transcript scrolls, five rows a notch, and it stops at both ends. A slow
   two-finger slide creeps at the speed of the fingers instead of sticking, and a
   wheel notch on a modern compositor moves as far as on an older one. Spin it: the
   first notch is five rows and each one after it travels farther, up to four times
   five rows, and holding still for a moment brings the next notch back to five
   rows. One two-finger slide follows the fingers exactly however fast it is; three
   quick flicks in a row each go farther than the one before. A 3 px
   indicator on the card's right edge comes up with the scroll, holds a moment, and
   fades out about a second after you stop; it is not on the card at rest. Spin fast
   enough for the burst and that strip becomes a capsule in its place reading `1.5x`,
   `2x`, on up to `4x`, and it is the plain strip again 400 ms after the last notch
   while the strip itself fades on its own clock. One notch, or one slide, is only
   ever the strip. Over the
   bar — its first row and its last — and in the gap just above it, nothing scrolls;
   with an answer streaming, rest the pointer there and flick: the newest line stays
   in view and stays in view when the answer ends. Mid-scroll, no two lines ever
   overlap: the line at the card's bottom edge fades out instead of being cut off.
5d0b. Press in the card's right padding, on the indicator, and drag it to the top of
   the card: the replies show the transcript's first line, and an answer that is
   still streaming no longer pulls them down. Drag it to the bottom: the newest line
   is back and following resumes. Press well below the thumb and drag: it lands
   centred on the pointer rather than jumping. Hold it still for a few seconds: it
   stays at full ink while the button is down, and fades once you let go. Press the
   same padding while the transcript fits: nothing is grabbed and nothing scrolls.
5d. Ask something long enough to overflow the box: the panel is the same size it
   was, the newest lines are the ones on screen, and scrolling reaches the older
   ones.
6. While an answer is streaming, run `pgrep -a bwrap` in another terminal: the
   sandboxed process is there. Press `Esc`: the stream stops, `cancelled` appears,
   the panel stays open, and `pgrep -a bwrap` now finds nothing.
7. Press `Esc` again: the panel closes, and faster than it opened — the collapse to a
   line takes about a fifth of a second. Repeat the keybinding while it is open:
   the same panel closes and no second window appears.
7b. With the panel open, click a terminal behind it and type: the text goes to the
   terminal, not to the panel. Click the panel and type: it goes to the panel.
7c. Put `font_size = "28"` and `font_family = "DejaVu Sans Mono"` in the config and
   start again: the text is that size and that face, the bar is taller, and Chinese
   still renders. A `font_size = "2"` refuses to start, naming the file and line.
7d. Put `animations = "false"` in the config: summon, ask, close — each is one
   frame, with no spreading line, no unfolding card and no collapse; a slow request
   still lights the rim. `animations = "sometimes"` refuses to start, naming the
   line and both accepted values.
7e. With two outputs, run `aside --output <name of the second>`: it opens on that
   output at that output's gap, and moving the pointer to the first leaves it there.
   With no flag it opens where the compositor puts it, and a click on the other
   output does not close it. `--output nonsense` writes one line naming the outputs
   that exist and opens nothing.

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
13. Start once with `font_size = "72"` and ask something long: the rows, the cards and
    the scrolling follow the size, lines never overlap, the last line still fades into
    the card's bottom edge, and the 64-row cap holds on a small output.
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
    naming `OPENCODE_API_KEY`, non-zero exit, and no window.
