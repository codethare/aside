# Pin the panel to an output, and know its size before the first frame

## Why

`aside` creates its layer surface with no output, so the compositor places it — on
Hyprland that is the focused monitor, which is the right guess. Two things follow from
having no output at creation time:

The panel's geometry is read from the output it lands on, and that arrives as
`wl_surface.enter` after the surface is mapped — after the first frame has already been
painted at `FALLBACK_OUTPUT_H` (1080). The bar's gap is 38 % of the output height, so on
a 1440p screen the opening animation runs 137 px low and then jumps, and the same frame
also centres and caps the card at the wrong size. Omarchy's shell treats this as a
correctness problem rather than a wobble: its overlays keep their first frames
transparent until the output's geometry and scale are known.

And a session with two outputs has nothing to say which one the panel belongs on: an
`exec` from a keybinding cannot know, the compositor's choice cannot be overridden, and
a bug report about "the panel is on the wrong screen" has no answer.

## What changes

- `--output NAME` creates the layer surface on that output. The output's `wl_output`
  is a create-time argument to `zwlr_layer_surface_v1.get_layer_surface`, so this is
  the only moment it can be chosen. A name nothing provides is one line naming the
  names that do exist, and no panel.
- Before the first commit the process asks the compositor for the output list (one
  roundtrip) and takes the chosen output's logical size. With no `--output` and
  exactly one output, that output is the chosen one. The first frame is then drawn at
  the gap, width and height the screen actually has.
- Everything after that is unchanged: the panel is still placed where the compositor
  puts it when nothing is pinned, and still takes its size from `surface_enter`.

## Cost

One blocking roundtrip before the first commit, at a point where the font is loading
on its own thread anyway. A session with several outputs and no `--output` still opens
at the fallback height for one frame, because the client cannot know which output the
compositor will pick; the alternative is to hold the panel back until `surface_enter`
arrives, which is a guess with a timeout on compositors that never send it before the
first buffer.

A click on another output still does not close the panel. Omarchy buys that with a
transparent twin surface on every other screen; a second surface, a second buffer and a
second exit path are more than "which screen am I on" is worth here, so it stays a
known limit.

## Impact

- `src/main.rs`: the flag, the roundtrip, `boot` (choose, re-create the surface, note
  the size, commit) and a pure `choose_output` with its test.
- README, man page, the live checklist.
- No configuration, no dependency, nothing written to disk.
