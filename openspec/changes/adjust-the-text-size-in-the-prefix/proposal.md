# Change the text size while the panel is open

## Why

The panel's size comes from one number, `font_size`, read from the configuration file
at startup and never again: 8 to 72 logical px, and the bar, the row height, the gap
and the wrap all follow it. Changing it means editing a file and restarting the
process, which for a panel that is summoned for one question is the wrong price for
"it is a bit small on this screen".

The prefix mode exists for exactly this. `Ctrl+T` waits for one key, the bar shows what
it is waiting for, and `src/main.rs` says of the table behind it: *"The table is the
place later settings go"*. Today that table has one entry, `[`, for copy mode.

Omarchy's shell keeps every size on a scale that can be changed while everything runs —
`omarchy display text size` pushes a `shell.toml` the shell is watching, and every
token derives from `base-size`. `aside` has no file to watch and no state on disk, so
the equivalent is the key itself.

## What changes

- The prefix gains two keys: `+` (and `=`, which is where `+` lives on most layouts)
  for larger and `-` for smaller. Two logical px a press, clamped to the existing
  `8..=72`, and a press at the limit changes nothing.
- The new size takes effect at once: the size in `font.rs` becomes mutable, the font is
  read again at the new size, the transcript is laid out again, and the surface is
  resized through the same `apply_size` path every other size change uses. A reply on
  screen stays where it is and is re-wrapped at the new metrics.
- The size is not written down. It lives as long as the process, like the panel's
  position and the transcript.
- The prefix's own hint line grows a clause, so the keys are discoverable from the bar.

## Cost

The font is read again on the keypress — one file read and a face parse, a few
milliseconds — on the event loop, so a press is followed by a brief pause before the
next frame. Doing it on a thread like the first load would show the old metrics at the
new bar height for a frame or two, which is a worse lie than the pause.

## Impact

- `src/main.rs`: the prefix table, its hint, `font_step` and the re-derivation; a test
  for the keys and the clamp.
- `src/font.rs`: the configured size stops being a `OnceLock` and becomes readable and
  writable.
- README, man page and the live checklist.
- No new dependency, no new file, nothing written to disk.
