---
name: add-a-panel-mode
description: Use when adding another look or chrome to the aside panel — a new `mode` value beside `default` and `cat`, or a change to how a mode is drawn, staged, or switched. Also use when a mode's layer surface, its coordinates, or its buffers are involved.
---

# Adding a mode to `aside`

A **mode** is one look for the same panel, chosen by the `mode` key in
`~/.config/aside/config.toml`. The panel's surface, its two cards, its text, its keys,
its scrolling, its selection, its markdown, its request and its sandbox are the same in
every mode; a mode changes what is drawn, where, and — if it needs a second surface —
what is painted beside the cards.

`default` is the plush panel and must keep drawing exactly what it drew before. Every
new look is a new value, never an edit to `default`.

## The procedure

1. **Open an OpenSpec change** under `openspec/changes/<kebab-name>/` with
   `openspec new change <name>`. The behaviour lands in `specs/` before any code: a
   delta on `chat-panel` for anything the panel draws, and a new capability (for
   example `cat-stage`) for anything that owns its own surface or its own animation.
2. **Design first, and get it approved.** `design.md` carries: intent, goals,
   non-goals, the constraints you found in the code, two or three approaches with what
   each costs, the one you picked and why, and a *landing identity* — the geometry the
   mode's parts meet at, expressed as an equation a test can check. A design that says
   "the paw sits next to the input" is not a design; `stage.x + stage.size == card_left`
   is.
3. **`tasks.md` is the plan**, one task per deliverable, each with Files, Interfaces,
   and RED→GREEN steps. Interfaces are how neighbouring tasks learn each other's names;
   a task that consumes an earlier one gets a row in the pre-flight scan.
4. **Execute inline with `superpowers:executing-plans`**, TDD per step, a ledger line
   per task, a `Ruling:` line for every deviation. The ledger is what survives
   compaction; the commits are what survive the session.
5. **Gate, unpiped, every task and at the end.**
6. **One whole-branch review** from a fresh reviewer, with a *Review Focus* list of the
   inputs your tests do not exercise. Fix what it finds in one pass, each fix with a
   test watched failing first.

## What a mode touches

| File | Its part of a mode |
| --- | --- |
| `src/provider.rs` | the key's validation in `parse_config`, with its line number |
| `src/main.rs` | `Skin`, `skin()`, `Options.skin`, `State.skin`; the surface lifecycle; the pose clock |
| `src/panel.rs` | `Look.skin` (the cards) and `Look.mouth` (where a card grows from) |
| `src/cat.rs` | the mode's own geometry, poses and painter, if it has them |
| `src/transcript.rs` | only if the mode keeps a turn on screen longer than the plush panel does |
| `README.md`, `docs/aside.1`, `CLAUDE.md` rule 4 | the key, the look, and the chrome rule's scope |

## Traps this cost a reviewer six findings to find

### Two origins, and only one of them is yours

The panel's buffer starts at the panel surface's origin on the output; a second
surface's buffer starts at its own. A point that crosses between them is
`(place.x - panel.x) * scale + local`, never `place.x * scale + local`. Getting this
wrong does not look wrong — it writes past the end of the buffer and panics, or paints
a card in the wrong place. Keep the conversion in one named function
(`mouth_in_surface`) and test that its result is inside the panel's buffer.

### Any buffer is keyed to a size and a scale

A shm buffer allocated at 260 px is not a buffer at 520 px, and `Ink::px` checks the
dimensions you *tell* it, not the slice's length. Drop the buffer whenever the frame's
pixel size changes (`buffer_fits(stage.buffered, px)`), and set the second surface's
`set_buffer_scale` wherever the panel adopts a new scale — a resize, a zoom, a
fractional-scale change.

### A seam is an equation, and it needs whole pixels

Two surfaces meeting at a contact are one point only if the contact is a whole buffer
pixel at every integer scale. Give the window an **even** side (`want - want % 2`), so
`size * scale / 2` is an integer for every scale, and test the identity at scales 1, 2
and 3 and at an odd card height. An odd side is a one-pixel step between the paw and the
pad that no one can name and everyone can see.

### Keeping a turn on screen is a transcript fact, not a shape fact

`shape_after` already keeps the reply card through `Thinking`, which makes it tempting to
believe the bubble stays. It does not: `send` retracts a visible answer, `begin_request`
sets `shape = State::Bar` unconditionally, and `ask()` retires the previous turn's layout.
Three places decide it, and all three must agree: `Transcript::ask(question, font,
keep_previous)`, the retraction guard in `send`, and the `shape` assignment in
`begin_request` (see `keeps_bubble`). Grep for every write to `self.shape` before
believing a shape survives a phase change.

### Reconcile a surface's existence, not only its place

A window that is placed every frame must also be *unmapped* when it no longer fits
(`hide_stage`) and *re-created* when the room comes back. Otherwise the cat is left
standing where it no longer belongs, and never returns after the panel is dragged clear.

## The gate, and this machine's four non-failures

```
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
groff -man -Tutf8 docs/aside.1 > /dev/null
```

**Never pipe a gate command through `tail` or `head`**: the pipeline's exit status is
the last command's, so a red clippy reads as success. Redirect to a file and check `$?`.

On a machine without a CJK font, without a Wayland session and without `groff`, these
are *not* your regressions, and they must be reported as pre-existing rather than
inherited:

- `font::tests::fallback_chain_covers_cjk_for_latin_primary` — there is no CJK face to
  fall back to.
- `panel::tests::a_long_question_scrolls_to_keep_its_caret` — its ink columns depend on
  the metrics of the installed face, and it fails on a machine whose primary face is not
  the author's.
- `sandbox::tests::a_network_request_still_works_from_inside` — makes a real HTTPS
  request, so it fails intermittently.
- The man page check cannot run without `groff`.

With no Wayland session the surface path is unverified, and that is the mode's real risk:
creation, configure, margins, the empty input region, the follow on drag and resize, and
the stacking of two layer surfaces. Say so instead of implying it works.

## The art loop

`cargo run -- --preview /tmp/aside.ppm` writes the panel, `.cat.panel.ppm` (the pad and
the bubble), and one `.cat.<pose>.ppm` per pose — `peek`, `hop`, `arm`, `waiting`,
`lick`, `speak`, `extend`. Convert the PPM to PNG and *look at it*: a silhouette that
reads in your head does not read at 260 px. The cat in `aside` needed three passes before
its head sat on its body.

## Red flags

- A mode that changes what `default` draws.
- A coordinate that crosses two surfaces without subtracting an origin.
- A buffer kept across a resize or a scale change.
- An odd window side, or a contact tested only at scale 1.
- "The shape keeps the card, so the card stays" — check every write to `self.shape`.
- A gate command whose output you piped.
- A surface placed but never unmapped.
- A claim that the surface path works, from a machine with no compositor.
