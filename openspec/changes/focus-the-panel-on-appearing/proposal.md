# Take the keyboard when the panel appears

## Why

The panel could only be typed into after clicking it. It is a `wlr-layer-shell`
surface with `OnDemand` keyboard interactivity, and that is what the protocol
means: the keyboard arrives when the surface is clicked. Nothing else offers one —
niri's `layer-rule` has no focus option, sway's `for_window` matches containers
rather than layer surfaces, and `xdg-activation` is a no-op for a surface that is
not a toplevel — so a second summon only redrew, and the pointer was the only way
back to the input line.

`Exclusive` is the one interactivity mode in which a compositor hands a layer
surface the keyboard on its own: the seat gives exclusive keyboard focus to the
top-most overlay-layer surface that asks for it, which on map is this panel. The
process exits when the panel closes, so every summon is a fresh map.

## What changes

`layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive)`, as one
constant `INTERACTIVITY` with the reason beside it, and a test that fails if it
goes back to `OnDemand`.

Nothing else: not the surface type, not the placement, not the keys, not the
movement.

## Trade

The other side of exclusive focus: while the panel is up the keyboard stays with it,
so a window behind it can be clicked but not typed into. `Esc` closes the panel and
gives the keyboard up. This is the accepted cost of never needing a click to type a
question.

## Rejected: making the panel an ordinary window

An `xdg-toplevel` would get focus and placement from the compositor and its window
rules, and it was implemented and pushed before being reverted: it loses the overlay
stacking, the workspace independence and the bottom anchor, needs a float rule on
every tiling compositor, and hands the panel's geometry to a window manager that
was not being asked for it.
