# A second summon puts the panel away

## Why

The second press of the summoning key had nothing left to do. Raising was for a
toplevel window that could be buried, and a layer surface on the overlay layer is
never behind anything: the old path asked for an `xdg-activation` token, which is a
no-op for a surface that is not a toplevel, and then redrew a panel that was already
on screen. The only thing left for the second press to mean is "not now".

## What changes

- A second summon **closes the panel** — the closing `Esc` starts — when the panel
  holds the keyboard, and **takes the keyboard back** when it does not. The new launch
  exits zero either way. A summon that arrives while the closing is already running
  changes nothing.
- The `xdg-activation` path goes with the raise: there is nothing left to raise, so
  `ActivationState`, its handler, its import and the `raise` method are deleted, and
  the control socket's byte means toggle.

## The keyboard, which this change first got wrong

An earlier version of this change also made the panel map asking for `Exclusive` and
drop to `OnDemand` 75 ms after the first configure, borrowing the *prime* from
omarchy's `shell/Ui/KeyboardPanel.qml`. It shipped, and the panel could not be typed
into at all. Hyprland's own source says why, in two places:

- A layer surface whose interactivity is anything but `none` takes the keyboard when
  it maps (`src/desktop/view/LayerSurface.cpp`, `GRABS_KEYBOARD` → `takeKeyboardFocus`).
  So `OnDemand` alone is already typable on a fresh map, and a panel that exits when
  it closes only ever has fresh maps.
- Changing the value on a mapped surface is what breaks it: on an
  `Exclusive` → `OnDemand` commit Hyprland calls `simulateMouseMovement()`, and with
  the pointer over another window that hands the keyboard to that window. The panel
  stays on screen and stops accepting keys.

So the interactivity is asked for once, at creation, and never changed: `OnDemand`,
with the reasoning written beside the constant. Omarchy's prime exists for panels it
keeps mapped between summons; aside remaps on every summon, and the prime is not a
pattern that survives the difference.

## The other half: getting back

Closing on the second press alone is wrong for a layer surface, and river shows why.
Its `river_seat_v1` can focus a window or a shell surface the *window manager* created;
it has no request that focuses another client's layer surface, and a layer surface is
given the keyboard only at its map (or, on river, when the pointer interacts with it).
So once focus goes to a window, a keybinding has no way to bring the panel back — and
the panel used to answer that press by closing, which threw the conversation away.

The second summon therefore reads the keyboard: with it, the panel goes away; without
it, the panel takes it back. Taking it back is an unmap/map of the surface, which is the
only lever a layer-shell client has. That is why this change keeps the toggle rather
than the raise: `raise` would need `xdg-activation`, which is a no-op for a layer
surface, and a fresh map is a different thing — it is the only thing that works.

What remains out of reach: a pointer that merely hovers over the panel (a cursor mover
like `windhover`) does not focus it, because river focuses a layer surface on pointer
*interaction*; and no window switcher will ever list the panel, because layer surfaces
are not windows. Both would need the panel to be an `xdg_toplevel`, which is a different
product decision than the one this panel is built on.

## Cost

A compositor that does not focus an interactive layer surface when it maps still gives
the panel the keyboard only when the pointer reaches it — the state `share-the-keyboard`
left the panel in, unchanged by this change. Nothing gets worse: the composite value
is the one `share-the-keyboard` chose, and no surface has to be upgraded and downgraded
mid-life.

## Impact

- `src/main.rs`: the socket handler's meaning, the deleted activation code, and the
  comment that records why the interactivity is set once.
- `src/control.rs`: the claim is a toggle request, not a raise request.
- README, man page, `CLAUDE.md` rule 3, the live checklist.
- No configuration, no dependency, nothing written to disk.
