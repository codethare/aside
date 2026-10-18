# CLAUDE.md

Binding rules for every change in this repository. Each rule is checkable; if it
cannot be verified, it does not belong here.

## What this is

`aside` is a keyboard-summoned AI chat panel for Wayland: press a key, type a
question, watch the answer stream in, ask a follow-up, take a line out to the
clipboard. The transcript lives in RAM and dies with the process.

Behaviour is specified in `openspec/changes/add-aside-chat-panel/specs/`. Read the
spec before changing behaviour; a behaviour change updates the spec in the same
change.

## Product rules

1. **Nothing on disk.** No history, no cache, no state directory, no log file.
   The only file ever read is the config, and it is never written. See
   `specs/ephemeral-conversation`.
2. **The provider request runs sandboxed.** One `curl` inside one `bwrap`, with
   the network and nothing else, and the credential on stdin rather than in
   `argv`. See `specs/chat-sandbox`.
3. **Keyboard first.** One meaning per key; every action reachable without a
   pointer. The key table in `specs/chat-panel` is the contract.
4. **The UI is `wlmenu`'s UI.** Tokens are copied exactly: radius 12, `PAD` 16,
   4 px rail, 8 px gap, 2 px pill, 1 px divider, the colour constants in
   `render.rs`. Row height stays `size + 2 * round(size * 0.25)`.
5. **Frame cost is per visible row**, never per transcript row. Wrapping happens
   once per delta and once per resize; re-wrapping on every frame is a bug.
6. **Zero dependencies beyond the six in `Cargo.toml`.** Ask before adding one;
   prefer deleting code to adding a crate.
7. **Every failure is visible and exits.** No protocol error may become a busy
   loop, and no error path may leave a process with no way to be seen.

## Code style

- Single-file modules, pure functions and constants, BGRA colours, ASCII source.
- Software rendering only: `wl_shm` + `skrifa` outlines + `ab_glyph_rasterizer`
  coverage. No cairo, pango, skia or GTK.
- Comment the why, in one line, beside the thing it constrains.
- Only change what was asked; neighbouring modules are not yours to tidy.

## Development flow

- Spec first: behaviour lands in `openspec/changes/<change>/specs/` before code.
- Tasks live in that change's `tasks.md`, one checkbox per task, ticked as it
  lands. Nothing is ticked while it is partial.
- Each non-trivial change carries an assertion-style test in the module it
  touches. No test framework.
- Gate: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings`.

## Boundaries

- Always: the gate passes; the specs still describe the code; the credential
  never reaches a log line, an `argv` or the panel.
- Ask first: adding a dependency; changing a key binding; changing a token value;
  changing what the sandbox exposes.
- Never: write conversation content anywhere; run `curl` without `bwrap`; delete
  or loosen an existing test.

## Version control (jj)

- `jj status`, `jj diff`, `jj describe -m "..."`, `jj new`. English messages,
  imperative subject under 72 characters: `feat:`, `fix:`, `docs:`, `test:`.
- The repository's previous history (a `quickbar` bar) is abandoned and unused.

## Known limits

- No complex-script shaping: Arabic, Devanagari and Thai render unshaped.
- No `wp_fractional_scale_v1`; scale is whatever integer `wl_output.scale` says.
- No Markdown, code blocks or tables: answers are plain wrapped text.
- The sandbox does not filter the network: the provider sees the question.
- `wl-copy` keeps a process alive holding whatever was copied to it.
