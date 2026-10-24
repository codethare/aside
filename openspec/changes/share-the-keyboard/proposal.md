# Share the keyboard with the rest of the session

## Why

With the panel up, no other window could be typed into. Switching to another window
from the compositor's own binding left the keys going to `aside`, so the panel
behaved like a lock screen for as long as it was on screen.

The cause is the keyboard interactivity the panel asked for. `wlr-layer-shell` has
three modes, and `exclusive` is a grab: a compositor keeps an exclusive layer
surface focused and **undoes** every attempt to focus a view while it is mapped.
Sway does it in one place — `seat->has_exclusive_layer` makes `seat_set_focus` drop
the layer, set the view, and then re-set the layer — so no window rule, binding or
click can win while the surface exists. Only `on_demand` leaves the keyboard
shareable: the compositor focuses the surface when the pointer is over it
(focus-follows-mouse) or when it is clicked, and takes it away again when something
else is focused.

## What changes

- `INTERACTIVITY` becomes `OnDemand`, and the test that asserted the old grab now
  asserts the keyboard stays the compositor's to move.
- The requirement it came from, `focus-the-panel-on-appearing`, is reversed by this
  change; its reasoning was sound about the protocol (nothing focuses a layer surface
  on its own) but the cost it accepted - a session where no other window can be typed
  into - is not one the panel is worth.
- Nothing else: the keys, the modes, the IME handshake and the exit path are the
  same. Losing focus already disables the text input and drops a pre-edit, and
  regaining it enables the input again, so the panel goes quiet when it is not
  focused and is typable the moment it is.

## Cost

The panel is no longer typed into merely by appearing: a compositor that does not
follow the pointer (or a session with focus-follows-mouse off) needs one click on
the panel first. That is the price of the keyboard not being grabbed, and it is the
compositor's choice, which is where it belongs.

## Impact

- `src/main.rs`: the constant, its comment and one test.
- README, the man page, `CLAUDE.md` rule 3 and the live checklist say what the
  session does now instead of promising a grab.
- No configuration, no dependency, nothing written to disk.
