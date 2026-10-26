# Tasks

Phases are independent enough to land one at a time, in this order.

## 1. A span is a set of styles

- [ ] 1.1 `Styles { strong, em, code, link }` replaces `Style` on `Span`; `layout::style_at` becomes `styles_at`
- [ ] 1.2 `markdown` keeps an inline stack and merges adjacent spans only when their style sets are equal
- [ ] 1.3 `panel` composes one `Look` per span (`face`, `ink`, `chip`, `underline`) in one function
- [ ] 1.4 A test per composition: `**a *b* c**`, a code span inside a link, a link inside strong, and a marked row collapsing to one ink

## 2. A slanted face

- [ ] 2.1 `font`: an italic chain read lazily, `ITALIC_CANDIDATES` first and a database query after
- [ ] 2.2 `None` when the system has no italic face, and emphasis then takes its own ink weight
- [ ] 2.3 Tests: the italic face is not read for an answer with no emphasis; a slanted face is used when one exists; the fallback is still distinguishable

## 3. Marks that are not text

- [ ] 3.1 An inline code chip: `BG_CODE` behind the span, two pixels either side, measured with `row_advance`
- [ ] 3.2 A link underline: one pixel under the span, in the dim ink
- [ ] 3.3 A rule row: one hairline across the card's text width instead of an empty row
- [ ] 3.4 Tests: the chip is as wide as its span, the underline sits under the span and not under the row, the hairline reaches both ends of the text width, and a marked row draws no chip

## 4. Text the parser hands over verbatim

- [ ] 4.1 Tabs expand to the next multiple of four at layout time, in code and in prose
- [ ] 4.2 One fence rule shared by the renderer and the settle point: three or more, at most three spaces of indent, a closer at least as long
- [ ] 4.3 Everything outside the subset is still drawn as written: a table, a task list, front matter, raw HTML
- [ ] 4.4 Tests: a tabbed block stays inside the card and lines up; an indented fence and a long closer; a table survives a round trip through the rows

## 5. What the keyboard walks

- [ ] 5.1 `move_highlight` and the copy-mode motions step over rows with no text
- [ ] 5.2 A copy spanning rows keeps the empty rows between them
- [ ] 5.3 Tests: `Down` from the last row of a block reaches the next block's first row, `Up` the same in reverse, and a copy across two blocks keeps the blank line

## 6. Docs and gate

- [ ] 6.1 `README`, `docs/aside.1`, `CLAUDE.md`: emphasis is no longer listed as unstyled, a rule is no longer an empty row
- [ ] 6.2 `docs/live-checklist.md`: an answer with all of it at once, checked on screen and copied out
- [ ] 6.3 `--preview` shows every mark in this change
- [ ] 6.4 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`, `cargo audit`
