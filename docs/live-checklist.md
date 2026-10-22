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
   centred, its field just below the vertical centre, and takes the keyboard.
2. Type `what does EAGAIN mean` and press `Enter`. The ball spins, then the
   reply card is seen to rise out of the bar, streaming as it grows; the bar
   itself does not move.
3. Ask a follow-up. The earlier answer stays on screen (your questions are never
   drawn), the bar does not move, the panel grows upwards, and the new answer
   takes the earlier exchange into account.
4. `Up`, `Down`, `PageUp`, `PageDown` move the highlighted line and the view
   follows it; the highlighted line is brighter than its neighbours and starts
   2 px further right, and nothing is drawn in the left margin.
5. `Alt+C` copies exactly the highlighted line. Paste it somewhere and check
   there is no extra whitespace or transcript around it.
5b. `Ctrl+T` then `[`: the bar reads `-- VIM --` and the caret is gone. `hjkl`
   move the panel (16 px), `Ctrl+hjkl` move it 160 px, and it stops at the screen
   edges; `n`/`p` page the transcript, `Ctrl+n`/`Ctrl+p` move one line, `g`/`G` go
   to the first/last line. No letter reaches the input line. `Esc` leaves the
   mode, and a second `Esc` cancels a stream that was running the whole time.
5c. Hold the left button for a quarter of a second and move: the panel follows
   the pointer in both directions, stays on screen, and does not change size while
   it moves. A short click moves nothing, and a new run starts where the anchor
   puts it.
5d. Ask something long enough to overflow the box: the panel is the same size it
   was, the newest lines are the ones on screen, and scrolling reaches the older
   ones.
6. While an answer is streaming, run `pgrep -a bwrap` in another terminal: the
   sandboxed process is there. Press `Esc`: the stream stops, `cancelled` appears,
   the panel stays open, and `pgrep -a bwrap` now finds nothing.
7. Press `Esc` again: the panel closes. Repeat the keybinding while it is open:
   the *same* window comes forward, and no second window appears.

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
    shared tokens against each other: radius, padding, the 12 px card separation,
    row height, the glass tone, the shadow falloff, the hairline profiles and the
    corner antialiasing. This is the side-by-side check the design asks for, and
    it needs a real `wlmenu` frame from your session.
14. Scroll back while an answer is streaming: the view stays where it was put.

## Failure paths (optional, each costs one request)

16. Ask a question with `ASIDE_MODEL=nonsense/model`: the transcript shows a
    provider error naming the status, and the panel stays usable.
17. Unset the key and run `aside` from a terminal: one line on standard error
    naming `COMMAND_CODE_API_KEY`, non-zero exit, and no window.
