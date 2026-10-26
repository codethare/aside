# Let the session turn the panel's animation off

## Why

The panel is animated everywhere: it opens by spreading a line out of a point and
unfolding a card up out of it (520 ms), a reply grows the bar into the card (460 ms),
and closing runs the opening backwards (200 ms). Every one of those is a moving shape
in the middle of the screen, and there is no way to say "not for me": not in the
configuration, not with a flag, not from the session.

Omarchy's shell treats this as a session-wide fact rather than a per-widget choice: a
theme token `Style.duration(ms)` is where *every* animation length goes, and it returns
0 when the compositor's animations are switched off — read from
`hyprctl getoption animations:enabled` and re-read on `configreloaded`, so the shell
follows the setting without being told twice. `aside` has no compositor-specific
awareness and should not grow one; it has a configuration file, which is the one thing
it reads.

## What changes

- One new configuration key, `animations`, `"true"` or `"false"`. Anything else is one
  line naming the file and the line, and no panel — the same contract every other key
  has. Absent means animated.
- With it `false` the three transitions are skipped, not shortened: the tracks of the
  opening and the closing are already at their end, and a shape change has already
  settled, so the panel appears, changes shape and goes in a single frame each. The
  working light on the rim and the scroll indicator's fade stay: they are cues that
  something is happening, not the panel moving.
- The reading is one helper, `clock_ms`, at the three places where a duration is
  compared against elapsed time. Nothing else in the state machine changes.

## Cost

One more key in a file that already has five, and no runtime path to turn animation
back on: like every other key it is read once, at startup. The alternative — reading
the compositor's own setting — would put a `hyprctl` call, or a Hyprland-only IPC
dependency, inside a program that speaks nothing but Wayland.

## Impact

- `src/main.rs`: the key's reading, one `State` field, `clock_ms` and its three call
  sites, and a test.
- `src/provider.rs`: the key is validated where its line number is known, and a test.
- README and the man page.
- No dependency, nothing written to disk.
