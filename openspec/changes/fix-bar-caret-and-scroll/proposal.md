# Fix the bar's caret, its length and its entrance

## Why

Three defects in the input line, all of them the same kind: what the bar draws is
not what the input holds.

1. **The caret was never at the input's caret.** It was painted at the end of the
   text. `Left`, `Right`, `Home`, `End`, `Ctrl+Left` and `Ctrl+Right` moved the
   input's caret and `Backspace` deleted there, but nothing on screen moved, so
   correcting a word in the middle of a question looked impossible.
2. **The input line had a silent length limit.** The text was painted from the
   left with the card's width as its cap, so past one bar's worth of characters
   the line simply stopped being drawn while the input kept taking text.
3. **The line appeared before the bar did.** During the entrance the text was laid
   out against the card's *current* height, so on a two-pixel-tall card it was
   painted above the card, hanging in the air before the bar had unfolded to hold
   it.

## What changes

- The caret is drawn at the input's own byte index, so every caret key is visible
  and a backspace deletes where the caret is.
- A line wider than the bar scrolls, keeping the caret inside the field; the line
  is drawn from a character boundary with the partly scrolled glyph shifted, not
  dropped. No length limit is left in the painter.
- The bar's line is laid out at the bar's settled geometry and clipped to the rows
  the card has unfolded, so it forms as the bar rises instead of preceding it.

Nothing about the keys, the tone, the pre-edit, the label or the chrome moves.

## Impact

- `src/panel.rs`: `draw_bar`, one `band` helper and `scroll_to`; `draw_content` and
  the settled path pass the card's rows instead of a height.
- `src/render.rs`: `draw_text` takes the rows it may paint on (one extra clip in
  `blit_plane`), which also lets a reply row stop at its card's edge.
- No configuration, no new dependency, nothing written to disk.
