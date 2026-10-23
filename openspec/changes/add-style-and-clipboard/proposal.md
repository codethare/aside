# Proposal

## Why

Three things are missing or wrong. The panel's keyboard interactivity is
**exclusive**, which on a layer surface means the compositor never gives focus to
another window while the panel is up: clicking a terminal does nothing, and the
panel is not a modal dialog. Its text can leave by `Alt+C` but nothing can come in.
And the font — the one thing every user wants to change — is a compile-time
constant.

## What Changes

- **The panel no longer takes the keyboard away.** It asks for keyboard focus on
  demand: typed input reaches it while it is focused, and clicking another window
  moves focus there, as it should.
- **The clipboard works both ways**: `Ctrl+C` copies the highlighted line (the same
  as `Alt+C`, which stays), and `Ctrl+V` pastes the clipboard into the input line,
  read with `wl-paste` at the moment the key is pressed and nothing before.
- **Font size and family come from the config**: `font_size = "20"` and
  `font_family = "Noto Sans Mono"` beside `model`, `base_url` and `proxy`, with the
  bar height derived from the size so the whole panel follows.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: keyboard interactivity is on demand rather than exclusive; the key
  table gains `Ctrl+C` and `Ctrl+V`; the bar's metrics follow a configured font size
  and family.

## Impact

- `src/main.rs`: the interactivity mode, the two keys, the configured font.
- `src/font.rs`: the size and family, and a loader that takes both.
- `src/clipboard.rs`: reading the selection with `wl-paste`.
- `src/provider.rs`: two more config keys.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`, `CLAUDE.md`.
- One more program on `PATH` (`wl-paste`, from the `wl-clipboard` package the copy
  path already needs).
