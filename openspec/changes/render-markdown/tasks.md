# Tasks

## 1. The dependency, pinned

- [x] 1.1 `pulldown-cmark = { version = "= 0.13.5", default-features = false }` in `Cargo.toml`; the lock gains `pulldown-cmark` and `unicase` and nothing else
- [x] 1.2 A test feeding `"- [o]:u\n    \t"` through our render path: the panic 0.13.4 has in `into_offset_iter()` fails here if the pin is ever loosened
- [x] 1.3 `cargo audit` clean with the new lock (the advisory db has nothing for any of the four packages)

## 2. A row with styles, and offsets that do not move

- [x] 2.1 `layout::Row` gains `spans: Vec<Span>`, `indent: u32`, `code: bool`; its `text` stays the visible plain string
- [x] 2.2 A selection, a copy and the copy-mode cursor give the same characters as before: no `**`, no backticks, no fence markers
- [x] 2.3 `row_y` and `row_at` still answer `entry.y + index * row_h`: the grid is uniform

## 3. The renderer

- [x] 3.1 `src/markdown.rs`: parse with `Options::empty()`, build styled segments, wrap them into rows at `max_w - indent`
- [x] 3.2 Fenced blocks: monospace, `code: true`, hanging indent, wrapped, no label, no highlighting
- [x] 3.3 Inline code, strong, em, headings (bold + one empty row above), lists (bullet or number, `PAD` per level), quotes (indent + rail), links (text only), images (alt text)
- [x] 3.4 Anything outside the subset is drawn as written
- [x] 3.5 A turn's rows come from the renderer, re-run per delta and per resize; `retire()`, `current_start()` and `REPLY_CAP` behave as they do now

## 4. Drawing

- [x] 4.1 `panel::draw_row` paints span by span, with the selection band and the copy cursor still per character
- [x] 4.2 A code block's rows get the tinted background, 2 px inside the card, the way the selection band is drawn
- [x] 4.3 A quote's rows get their rail

## 5. The monospace face

- [x] 5.1 `font.rs`: a monospace chain beside the text chain, loaded lazily on the first code glyph
- [x] 5.2 A code row keeps `row_h` and the grid: no second row height

## 6. Docs and gate

- [x] 6.1 `README`, `docs/aside.1` and `CLAUDE.md` (known limits) say what is rendered and what is not
- [x] 6.2 `docs/live-checklist.md`: a real answer with a code block, a list, a heading and bold, checked on screen and checked by copying each
- [x] 6.3 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`, `--preview`, `cargo audit`
