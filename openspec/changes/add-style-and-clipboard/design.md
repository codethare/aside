# Design

## Context

`LayerShellHandler` sets `KeyboardInteractivity::Exclusive` at map time. On
wlroots that puts the surface in `seat->focused_layer`, and the compositor then
refuses to focus anything else until the layer is unmapped — which is exactly why
another window cannot be used while the panel is open. `key_action` has no `Ctrl+C`
and no paste; `clipboard.rs` only writes, by spawning `wl-copy`. `font::SIZE` is a
const, `panel::bar_height()` is `row_h_for(SIZE)`, and the font chain is a fixed
list of file paths.

## Goals / Non-Goals

- Focus that can move away; the clipboard in both directions; size and family from
  the file, with the panel's geometry following the size.

**Non-Goals:**

- Reloading the config while running, a per-element font (bar vs replies), font
  weight/style/antialiasing knobs, or a font picker.
- Mouse selection of reply text (`Ctrl+C` copies the highlighted line, which is the
  keyboard-first way to the same thing).
- Pasting into vim mode or a multi-line paste: the input is one logical line, and
  newlines in a paste become part of it exactly as typing them would.

## Decisions

**1. On-demand interactivity, and nothing else changes.** The panel never needed
exclusivity: it needs the keyboard while it has focus, which on-demand gives it
when it is clicked or summoned and gives back when another window is clicked.
*Alternative rejected:* exclusive plus exit-on-focus-loss — the panel is meant to
stay open while you use something else behind it.

**2. The clipboard is read on the key, never before.** `Ctrl+V` runs
`wl-paste --no-newline` and inserts what comes back; the panel holds the text only
in the input line, and nothing is polled or cached. A failure is reported in the
transcript like any other, so a missing `wl-paste` is visible rather than a silent
no-op.
*Alternative rejected:* a `wl-paste --watch` process — a long-lived reader of the
whole clipboard, which is a privacy liability for a panel that keeps nothing.

**3. One process-wide font setting, resolved once at startup.** `font::size()` and
`font::family()` read a `OnceLock` that `main` fills from the config; `load(size)`
stays and a `load_with(size, family)` takes the family. That keeps every existing
call site (`bar_height`, the preview, the scale change) working unchanged and makes
the configuration a startup fact rather than a parameter threaded through the
sizing functions.
*Alternative rejected:* a size parameter on `size_for`/`bar_height` — it would
touch every pure sizing function and every test for a value that never changes
while the process runs.

**4. The family is a preference, not a requirement.** A configured family is looked
up in `fontdb` and put at the head of the chain; if nothing matches, the built-in
chain is used as-is. A typo in a font name must not stop the panel from starting,
while a size that is not a size is a configuration error, because there is no
sensible fallback for "huge".
