# Tasks

## 1. Width policy

- [x] 1.1 Columns are measured by the code face's advance in space units, not by character count; a CJK, emoji or combining cell keeps its column starts (the existing CJK test plus the display-width comment)
- [x] 1.2 The header starts the widths and is fitted once; each body row can only add units while the card has room, round-robin across the columns, so no later row shrinks a column
- [x] 1.3 A table is not drawn until the header and separator arrive; a test pins that the partial text stays a paragraph
- [x] 1.4 A test renders the same table with one more long row and asserts every header column start moved right or stayed put

## 2. Fallback and copy

- [x] 2.1 If the minimum columns and gaps do not fit, the table becomes wrapped `header: value` lines; no right-hand column is clipped
- [x] 2.2 Every display row carries its logical table row and unpadded cells; a wrapped cell's rows carry the same data
- [x] 2.3 A whole-line copy of a table row emits TSV; a selection over wrapped rows copies the logical row once
- [x] 2.4 A table selection marks the whole display row, and every display row of the touched logical row, never half a cell
- [x] 2.5 Alignment (`:---`, `:---:`, `---:`) and the header rule stay as they were

## 3. Docs and gate

- [x] 3.1 README and the man page describe display widths, only-grow columns, the pair fallback and TSV copies
- [x] 3.2 `cargo test` (only the pre-existing font-environment failures), `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
- [ ] 3.3 A live check: a table in a streaming answer does not move its columns; a narrow panel shows pairs; copying a row pastes as columns in a spreadsheet
