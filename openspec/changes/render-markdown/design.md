# Design

## A row is text plus styles, and the text stays the text

The naughty part of this change is not parsing, it is that a row currently is one
string with one ink, and three subsystems index into that string by character:
the pointer selection (`Selection::columns`, `byte_index`), the copy
(`copy_highlight`, `copied_lines`), and the copy-mode cursor (`render::char_at`,
`render::advance`).

So the row keeps **one plain string**: exactly the characters the reader sees, in
order, with no markup. Styles are a parallel list of `(range, style)` over that
string's byte offsets. Interaction code keeps its character indices and its byte
offsets and never learns that styles exist; the painter walks the spans and picks
an ink per span. Copying therefore produces the visible text: no `**`, no
backticks, no fence markers.

```rust
struct Span { range: Range<usize>, style: Style }
enum Style { Plain, Strong, Em, Code, Link }
```

A row also carries `indent: u32` (buffer px, for hanging indents) and `code: bool`
(so the card can tint the rows of a fenced block).

## The grid stays uniform, so nothing about scrolling moves

`transcript::row_y` and `transcript::row_at` compute a position as
`entry.y + index * row_h`; the viewport, the highlight and the selection all
inherit that. Adding per-row leading would turn those into a search over stored
positions and touch every one of them. Not worth it.

Instead:

- extra separation comes from the grid itself — a heading is preceded by one
  **empty row**, which is a row like any other;
- indentation moves the row's `x`, and the renderer wraps at `max_w - indent`, so
  the grid is untouched.

Both are decisions the renderer makes once, when it lays the answer out.

## Special case: the wrap is per segment, not per turn

`Lines::push` appends a delta to a tail and breaks it at one width. Markdown needs
a different width per row (indent), and a style boundary that may fall mid-row, so
the answer path stops using `Lines`:

```rust
markdown::render(text: &str, font: &PanelFont, width: f32, row_h: u32) -> Vec<Row>
```

It parses the whole answer, builds a list of `(text, style, indent, code)` segments,
then wraps them into rows. It runs once per delta and once per resize — the same
two moments the current wrap runs — and `REPLY_CAP` bounds the input, so the total
is a few hundred microseconds per delta even though it is O(n) each time. The input
line and the questions keep the incremental `Lines`; they are not markdown.

## Blocks

| markdown | rows |
|---|---|
| fenced block | `code: true`, monospace face, wrapping at `max_w - 2 * PAD`, plus one `PAD` of hanging indent so a wrapped line stays inside the block |
| inline code | `Style::Code` on the span — monospace face, tinted ink |
| `**strong**` / `*em*` | `Style::Strong` / `Style::Em`: brighter ink; a slant only if the chain has a real italic face, never a synthetic one |
| heading | `Style::Strong` plus one empty row above (not above the first row of the answer) |
| list | a bullet `•` or the digits of the number, then the item's text indented by `PAD * level`; nested items are `PAD` per level |
| block quote | indented `PAD`, with a 2 px `LABEL_ACCENT` rail down the rows of the quote, at the row's own left edge |
| link | the link text, `Style::Link`, no URL drawn |
| image | the alt text, plain |
| anything else | its own text, unchanged: unknown syntax renders as written |

A blank source line inside an answer is an empty row; two in a row collapse to one,
so a model's generous spacing does not scroll the answer away.

## Fenced blocks and the font

Code needs even columns, and the panel has one text chain. `font.rs` gains a
monospace chain resolved beside it (`mono_chain()`), loaded the same lazy way: the
first code glyph loads it, so a panel showing plain prose never reads the file.
Code rows use the monospace face at the same size, so `row_h` and the grid do not
move; `Blocks.row_h` is unchanged.

Tinting: `render::BG_CODE` (a shade between `BG_PROMPT` and the shadow), painted
behind the block's rows inside the card, 2 px in from the card edge, the way the
selection band is painted. No language label, no highlighting.

## Streaming

An answer that ends mid-fence, mid-`**` or mid-list renders as the block it is
heading towards: `pulldown-cmark` closes an unterminated fence at the end of input,
and unclosed emphasis simply stays literal. The renderer runs on the whole answer
each delta, so the visible result is always the parse of everything received so
far — no incremental state to get out of step.

## What the implementation found

Four things only showed up once the renderer was running:

- **Blank lines are the separation.** A block is one empty row away from the block
  before it, list items excepted: the grid stays uniform, and a heading needs no
  rule of its own. The empty row is also what a `---` draws as.
- **`break_tail` was quadratic.** It measured the whole remaining string before
  walking it, so laying out a 64 KiB answer cost a full measurement per row. It now
  stops at the first character that overflows, which is one row's worth of work.
- **Word motion counted bytes.** `word_forward`/`word_back` answered byte offsets
  into the row while the selection, the copy and the copy-mode cursor count
  characters. With ASCII text the two agree, so the bug was invisible; the first
  bullet (`•`, three bytes) and any CJK row made `w` jump three columns. They count
  characters now.
- **The code face is read on the first backtick**, not when the font is built:
  reading it eagerly put a font-database scan in front of every start, and made the
  test suite take eleven minutes instead of four seconds. It is also resolved from a
  candidate path list before the database is asked, for the same reason the text
  chain does.

Per delta the cost is the open block, not the answer: a settled block is rendered
once. A single 64 KiB paragraph arriving in small chunks is therefore the worst
case, and it is bounded by `REPLY_CAP`; measured, laying out 64 KiB takes about
15 ms, so a stream that size would spend a few percent of its time laying out.

## See also

The dependency audit, the pin, and the panic this version fixes are in
`proposal.md`. The repro for it belongs in `markdown.rs`'s tests.
