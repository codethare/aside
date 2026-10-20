# Proposal

## Why

The answering panel puts the input bar above the transcript, so the panel reads
as a list with a search field on top, and every question the user typed takes a
row of screen to say something they already know. It should read as one answer
growing above the place they type: the bar below, the replies only.

## What Changes

- The answering panel is **the replies above the input bar**, like a summoned
  assistant's speech above its field: the bar is the panel's bottom row and the
  transcript the area above it.
- **BREAKING (layout)**: the panel is anchored to the **bottom** edge of the
  output instead of the top, 32 px up, so the bar keeps one position on screen
  while the answers grow upwards out of it. A top anchor would make the bar
  descend by a row with every wrapped line.
- The transcript draws **only what the model or the panel said**: answers,
  failures and notes. A question is no longer a row — it is kept in memory for the
  request and shown nowhere, so `Up`/`Down`/`Alt+C` move over reply lines alone.
- The idle panel and the thinking ball are unchanged: a bar, and a circle.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the panel is bottom-anchored; the bar is the bottom row of the
  answering panel; the transcript holds the replies only, keeping the questions
  for the provider without drawing them; the highlight and copy move over those
  lines.

## Impact

- `src/main.rs`: `Anchor::BOTTOM` and the bottom margin, the caret rectangle's
  new bar position, and the highlight set on send.
- `src/panel.rs`: the bar is drawn at the bottom, the divider above it, the
  transcript painted and clipped above, and the scroll indicator over the
  transcript area.
- `src/transcript.rs`: a question takes no rows and no height; `rewrap` skips it.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`, `CLAUDE.md` rule 4.
- No new dependency, no new file, nothing on disk.
