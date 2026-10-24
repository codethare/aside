# Make the panel a window the compositor manages

## Why

The panel cannot be focused again without a mouse click, and the reason is the
surface type. It is a `wlr-layer-shell` surface with `OnDemand` keyboard
interactivity, and that is exactly what the protocol means: the keyboard arrives
when the surface is clicked. No rule can focus a layer surface — niri's
`layer-rule` has no focus (or position) option, sway's `for_window` matches
containers rather than layer surfaces, and `xdg-activation` is a no-op for a
surface that is not a toplevel — so a second summon only redraws.

Client-side movement was the other half of the same problem: the panel moved itself
because the compositor could not place it. With the compositor owning the window,
that machinery is competing with the window manager instead of helping it.

## What changes

- The surface becomes an `xdg-toplevel` with `app_id` `aside`, no server-side or
  client-side decorations, and its own size. Placement, stacking, focus, workspace
  and window rules become the compositor's.
- `Alt+hjkl`, `Alt+Ctrl+hjkl` and the press-and-drag are removed: all of the code
  that moved the panel goes with them, and the keyboard contract loses those rows.
- The reply box keeps its fixed height, but the number comes from the output
  instead of a screen margin: 38 % of the output height, capped at 64 rows.
- A close request from the compositor exits, the way a closed layer surface did.

## Impact

- Focus: a new toplevel is focused on map by sway, niri and river, and
  `xdg-activation` now reaches the panel, so the second launch focuses it too.
- Lost: overlay stacking above everything, being outside workspaces, the bottom
  anchor, and the panel's own placement. On a tiling compositor the panel needs a
  float rule; the README documents one for each of the three.
- The 20 px (26 at scale) shadow margin stays inside the window, so the compositor
  adds its own window shadow around it.
