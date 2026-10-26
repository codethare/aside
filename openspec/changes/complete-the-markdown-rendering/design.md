# Design

## The span set, and why it is not a bitflag dependency

`bitflags` is already in the tree, but a four-field struct is clearer than a mask at
every use site and needs no import:

```rust
pub struct Span { pub range: Range<usize>, pub styles: Styles }

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct Styles {
    pub strong: bool,
    pub em: bool,
    pub code: bool,
    pub link: bool,
}
```

`markdown` walks the parser with an inline stack (`Strong`/`Emphasis`/`Link` push and
pop, a code span is terminal and sets `code` alone), and merges a new span into the
previous one when the ranges touch **and the styles are equal**. Two nested spans
therefore become two adjacent spans of distinct style sets, which is exactly what
the painter wants.

## One place decides how a style set looks

```rust
struct Look<'a> { face: &'a PanelFont, ink: Bgra, chip: bool, underline: bool }
fn look_of(font: &PanelFont, styles: Styles, kind: Kind, marked: bool) -> Look<'_>
```

- `marked` (the keyboard's line or a pointer's selection) collapses to one ink, as
  today: a selection reads as one piece.
- otherwise: ink is the primary one when `strong || code`, else the row's own; the
  face is the code face when `code`, else the italic face when `em`, else the text
  face; `chip` when `code`; `underline` when `link`.

Chips and underlines are painted from the same measurement the text uses
(`row_advance`), so a chip is exactly as wide as the span it sits under.

## The italic face

`PanelFont` already knows how to build a chain lazily (`CodeFace`). The italic face
repeats that shape:

- `ITALIC_CANDIDATES`: the sibling paths of the text candidates
  (`NotoSans-Italic.ttf`, `DejaVuSans-Oblique.ttf`, `LiberationSans-Italic.ttf`, …);
- failing that, a `fontdb` query for the configured family with
  `Style::Italic`/`Oblique`;
- failing that, `None`, and emphasis is drawn with the primary ink and no slant.

No synthetic shear: a skewed glyph in the cache is a second copy of every glyph, and
a fake slant on a hinting-free rasteriser reads as a printing error.

## Tabs

Expanding at render time, in `markdown`, before the text reaches a `Line`:

- column = characters since the row's start, `col + (4 - col % 4)`;
- in a fenced block the column counts characters too (no tab stops per font);
- outside a fence the same rule applies, so `\tafter` in prose lines up under the
  same column the wrapper measured.

The alternative — teaching `char_advance` and `draw_text` about tabs — would put a
font-dependent stop in the painter, and the wrapper and the painter would still have
to agree on the column, which is the bug being fixed.

## Fences

`boundary()` (the settle point) and the parser must agree about what closes a fence.
Both get one rule:

- an opener is a line whose first non-space run of three or more `` ` `` or `~` starts
  at most three spaces in, and (for `` ` ``) carries no backtick in its info string;
- a closer is a line of nothing but the same character, at least as long as the
  opener, with up to three spaces of indent and anything after it ignored.

`boundary` keeps its own small state machine because it runs before the parser does;
the rule lives in one function used by both.

## Rules

`---` becomes `Line { rule: true }`: no text, one pixel of `FG_NORMAL` at half alpha
across the card's text width, vertically centred in the row. It is a mark, like the
quote's rail, not a character, and it is the one place a block has no text — which is
why the row carries a flag instead of a styled `─` (a repeated glyph would depend on
the face having it and would not reach the card's edges).

## Navigation over empty rows

`move_highlight` and the copy-mode motions clamp to text: a row with an empty `text`
is stepped over, in whichever direction the motion is going, and the search stops at
the end rather than wrapping. A copy that spans rows still takes the empty rows
between them — the answer had those blank lines, so the clipboard shows them.

## What stays true

- A row's `text` is still the characters a reader sees; `selection`, `copy`,
  `word_forward/back` and the copy cursor still index characters of it.
- The grid is still `entry.y + index * row_h`; chips, underlines and hairlines are
  pixels inside a row, never a row of their own.
- `REPLY_CAP` still bounds the work: chips and underlines are per visible span.
