# Take the keyboard when the panel appears, and give it back

## Why

`share-the-keyboard` took the panel off `Exclusive` because an exclusive layer
surface is a grab: while it is mapped, a compositor undoes every attempt to focus a
view, so nothing else in the session can be typed into. That was right, and it left
the panel with only `OnDemand`, which by the protocol means the keyboard arrives when
the pointer is over the surface or when it is clicked. A hotkey-summoned panel that
has to be clicked first is what this project exists not to be.

Omarchy's shell hits the same wall from the other side and solves it by holding
`Exclusive` for one bound and then settling: a *prime*. `shell/Ui/KeyboardPanel.qml`
maps with `WlrKeyboardFocus.Exclusive`, a 75 ms timer flips it to `OnDemand`, and the
comment beside it says why both halves are needed — on-demand is what leaves the
keyboard shareable, and the prime is what gets it in the first place, including when
a surface that is still mapped for its fade-out is summoned again.

## What changes

- The layer surface maps asking for `exclusive`, and drops to `on_demand` once the
  compositor has configured it (`PRIME_MS`, 75 ms), or the moment the panel starts
  closing, whichever comes first. The grab is bounded by a timer, not by the life of
  the surface, which is the whole difference from the change this reverses.
- A second summon now **puts the panel away** instead of raising it: the key that
  opens it closes it. Raising was for a toplevel window that could be buried; a layer
  surface on the overlay layer is never behind anything, so the only thing left for
  the second press to mean is "not now". An open request goes with the process, as it
  does on `Esc`.
- The `xdg-activation` path goes with the raise: there is nothing left to raise, and
  an activation token is a no-op for a layer surface anyway. The `ActivationState`,
  its handler and its import are deleted.
- `README` gains the Hyprland `layerrule` recipe. The panel paints its own card and
  its own animation, so the compositor must not animate the surface a second time:
  `layerrule = no_anim, aside`.

## Cost

For 75 ms after the panel maps, a compositor that treats `Exclusive` as a grab will
not focus another view. The window is bounded twice — by the timer and by the closing
path — and nothing is drawn differently because of it. On a compositor that ignores
the interactivity change the panel behaves exactly as it does now.

The prime cannot be verified headless: it needs a real session and a real keybinding,
so it is a line in `docs/live-checklist.md` rather than a test. What is testable is
the shape of the contract, and that is what the test asserts.

## Impact

- `src/main.rs`: the interactivity constants, the prime's clock and its release, the
  socket handler's meaning, and the deleted activation code.
- `src/control.rs`: the claim is a toggle request, not a raise request.
- README, man page, `CLAUDE.md` rule 3, the live checklist.
- No configuration, no dependency, nothing written to disk.
