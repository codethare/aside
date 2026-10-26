# A second summon puts the panel away

## Why

The second press of the summoning key had nothing left to do. Raising was for a
toplevel window that could be buried, and a layer surface on the overlay layer is
never behind anything: the old path asked for an `xdg-activation` token, which is a
no-op for a surface that is not a toplevel, and then redrew a panel that was already
on screen. The only thing left for the second press to mean is "not now".

## What changes

- A second summon **closes the panel** — the closing `Esc` starts — and the new
  launch exits zero. An open request goes with the process that owns it. A summon
  that arrives while the closing is already running changes nothing.
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
