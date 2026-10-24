# Design

## The surface

`XdgShell::bind` + `XdgShell::create_window(&surface, WindowDecorations::None, &qh)`,
`app_id` and title `aside`. `WindowDecorations::None` asks the compositor not to
decorate and accepts client-side decorations if it insists; the panel draws no
titlebar either way, so a compositor that forces a titlebar shows it above the
panel's own 26 px shadow margin.

The first commit is buffer-less, exactly as the layer surface was: it asks for the
toplevel's configure, and nothing may be attached before that one arrives.
`WindowHandler::configure` replaces `LayerShellHandler::configure` and keeps the
same body — the configure is permission to attach, and the panel's own sizing
decides the size. `WindowHandler::request_close` replaces the layer surface's
`closed` event and sets the same exit flag.

## Size

The buffer size is still ours, and still what the phase asks for, so a window's
resize is the same one call it always was: a new buffer, no `set_margin`, no
`set_size`, no anchors. That removes `margins`, `left_margin`, `base_gap`,
`edge_gap`, `bottom_margin`, `dragged_left`, `dragged_bottom`, `drag_for_bottom`,
`Drag` and `move_panel`.

`room_for_replies` becomes a share of the output height:

```text
box = min(output_h * 38 / 100, MAX_ROWS rows)
```

It is the same number as before on a 1080-line output, and it no longer has to
reason about room left above an anchor — the compositor puts the window somewhere,
and a chat box that is 38 % of the screen fits wherever that is.

The panel's width stays 640 logical px and its text stays laid out at that width
(`layout_width`), so a compositor that tiles the window is answered by a float rule,
not by re-wrapping every answer.

## What goes

`Alt+hjkl`/`Alt+Ctrl+hjkl` (`Key::Window`, `window_move`, `WINDOW_STEP`), the
drag (`Drag`, `dragging`, `drag_to`, `Motion::Drag`), and the pointer's role in
moving the panel. `motion_with` keeps two answers instead of three: hover, or
extend a selection.

The pointer keeps everything else it had: press on a reply line starts a
selection, motion extends it with the button down, `Alt+C`/`Ctrl+C` copies it, the
wheel scrolls the replies.

## What this costs

A layer surface sits above every window, on every workspace, and is placed by its
own anchors. A toplevel is an ordinary window: it can be covered, it belongs to a
workspace, and its position is the compositor's. That is the trade the user asked
for — focus without a click — and it is why the README grows window-rule examples
instead of a section about moving the panel.

Rule 4 of `CLAUDE.md` loses the words "bottom-anchored `wlr-layer-shell` surface":
the chrome is unchanged, the surface is not.
