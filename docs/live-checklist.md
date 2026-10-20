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

1. Press the keybinding. The panel appears and takes the keyboard.
2. Type `what does EAGAIN mean` and press `Enter`. The answer streams in visibly,
   growing while the stream is open, and the 4 px rail beside the input is blue
   (`#0b6285`) until it finishes.
3. Ask a follow-up. Both turns stay on screen, and the answer takes the earlier
   exchange into account.
4. `Up`, `Down`, `PageUp`, `PageDown` move the highlighted line and the view
   follows it; the highlighted line is marked by a 2 px blue rail (`#0b6285`) in
   the left margin, not by a band across the row.
5. `Alt+C` copies exactly the highlighted line. Paste it somewhere and check
   there is no extra whitespace or transcript around it.
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
    shared tokens against each other: radius, padding, rail width, gap, divider,
    row height, the four colours, the shadow falloff, body translucency, the
    hairline border and the corner antialiasing. This is the side-by-side check
    the design asks for, and it needs a real `wlmenu` frame from your session.
14. Scroll back while an answer is streaming: the view stays where it was put.

## Failure paths (optional, each costs one request)

16. Ask a question with `ASIDE_MODEL=nonsense/model`: the transcript shows a
    provider error naming the status, and the panel stays usable.
17. Unset the key and run `aside` from a terminal: one line on standard error
    naming `COMMAND_CODE_API_KEY`, non-zero exit, and no window.
