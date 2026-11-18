# Proposal

## Why

Two kinds of text disappear instead of being marked.

`render` skips any character no face in the chain covers (`font.glyph` returns
`None` and both the advance and the ink are skipped). A script the default chain
does not include — Devanagari, Thai, Georgian, many emoji — is drawn as nothing, so
words run together and an answer can look corrupt. The chain is also built from a
short hardcoded path list plus one CJK face; a font that covers the missing script
is never consulted.

A tab is not a space to the renderer: no face maps `U+0009`, so it is skipped in
both measurement and drawing. Code blocks and pasted snippets lose their
indentation, and columns that should line up collapse.

## What Changes

- **A missing glyph is a box.** A character no chain face covers advances by the
  space's width and draws a hollow box, measured and drawn by the same rule, so text
  and pointer positions stay in step.
- **A tab is four spaces wide.** Both measurement and drawing advance four space
  widths and ink nothing, so indentations survive; the stored text keeps the tab.
- **Script fallbacks are loaded on demand.** The first missing glyph appends the
  system's script families (Devanagari, Thai, Bengali, …) and emoji faces to the
  chain, once, so a later lookup can render them. The font-database scan happens on
  that miss, never on the common startup path, and each face is read only when a
  glyph asks for it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: every character gets ink or a visible box, and a tab keeps its
  indent; the chain can grow to the system's script faces without delaying first
  paint.

## Impact

- `src/render.rs`: the missing-glyph box and the tab advance, shared by measure and
  draw.
- `src/font.rs`: the chain becomes growable; a first miss appends system script
  families, deduplicated, once.
- No new dependency, nothing on disk, no change to stored text.
