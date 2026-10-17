# Proposal

## Why

A question asked mid-work costs a browser tab: you leave the window you were
reading, lose your place, and the exchange lands in a history you did not ask
for. `aside` is a keyboard-summoned chat panel that answers on the spot, keeps
the transcript in RAM, and leaves nothing behind when it closes.

## What Changes

- New crate `aside`: one binary, a chat panel summoned by a compositor
  keybinding, streaming answers into the panel as they arrive and answering
  follow-up questions in the same transcript.
- The transcript lives in memory only: never a file, a cache, a config or a
  history, and gone when the process exits.
- The provider request is made by a `curl` child inside a `bwrap` sandbox that
  has the network and nothing else: no home directory, no session socket, no
  writable path. The API key reaches `curl` over a file descriptor, so it never
  appears in `argv`.
- Typed input goes through `zwp_text_input_v3`, so an IME (fcitx, ibus) composes
  text. Without it the panel is unusable for CJK prose.
- A line can be taken out of the transcript to the clipboard with `wl-copy`.
- Rendering, layout rules and design tokens are ported from `wlmenu`, so the
  panel reads as the same product rather than a lookalike.
- Default provider is Command Code, model `deepseek/deepseek-v4.1-flash`, with
  the credential read from `COMMAND_CODE_API_KEY`.

Out of scope for this change, and named here so they are not smuggled in:

- persistence of any kind: no history, no resume, no export;
- Markdown, code-block or table rendering, and syntax highlighting;
- a model picker, provider switcher or settings UI;
- restricting *where* the sandbox may connect (it gets the network unchanged);
- rendering of complex scripts that need shaping (Arabic, Devanagari, Thai).

## Capabilities

### New Capabilities

- `chat-panel`: the window itself — summoning and focusing it, the input line and
  its IME contract, the multi-turn transcript, scrolling, line selection, the
  copy path, and the key bindings.
- `provider-stream`: resolving the provider, model and key; assembling the request
  and running it through `curl`; decoding the provider's stream into ordered text
  deltas; mapping every failure onto a distinct, actionable message.
- `chat-sandbox`: what the `bwrap` containment of the provider process actually
  guarantees, what it deliberately does not, and how a run proves it held.
- `ephemeral-conversation`: the promise that no conversation content reaches the
  disk, and the evidence that it did not.

### Modified Capabilities

None: the project has no specs yet.

## Impact

- New crate at the repository root, Rust, `edition 2024`, toolchain pinned in
  `rust-toolchain.toml`.
- Dependencies: the same four `wlmenu` uses (`smithay-client-toolkit`,
  `wayland-client`, `ab_glyph`, `fontdb`) plus `wayland-protocols` for the
  text-input protocol. No HTTP or TLS crate: `curl` does the transport.
- Runtime requirements: a `wlroots`-family compositor with `xdg-shell`,
  `zwp_text_input_v3` for IME input, `bwrap`, `curl` and `wl-copy` on `PATH`.
  `bwrap` must be able to create a user namespace.
- No daemon, no socket, no state directory. The panel is one process that starts
  and dies with the conversation.
- Verification splits in two: parsing, layout, wrapping, sandbox argv and the
  no-disk check are headless and testable anywhere; the panel itself needs a live
  Wayland session.
