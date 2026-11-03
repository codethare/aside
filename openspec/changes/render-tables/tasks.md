# Tasks

Each task is RED first: the named test is written and run before the code that
makes it pass, and the run is read. Gate per task: `cargo test <name>` green, and
`cargo test` green at the end of the task.

## 1. Collect a table from the parser

- [x] 1.1 `Options::ENABLE_TABLES`; `Tag::Table(align)` / `TableHead` / `TableRow` / `TableCell` collected into `Table { align, head, rows }` with each cell's text and spans; `TagEnd::Table` emits the rows
- [x] 1.2 Each cell's whitespace collapses to single spaces and is trimmed
- [x] 1.3 Test `a_table_is_columns_of_padded_cells`: a 3×2 table renders as three text rows plus the rule; every row's column 2 starts at the same byte offset; no `|` in any row
- [x] 1.4 Test `a_header_is_followed_by_a_rule`: the row after the header is all `─`, its width covers the table

## 2. Fit the columns

- [x] 2.1 `unit = ceil(advance(mono, " "))`, `width[i]` = widest cell in units, floor `MIN_UNITS = 3`
- [x] 2.2 Shrink the widest column by one unit while `total > avail`, stop at `MIN_UNITS`
- [x] 2.3 Test `a_wide_table_is_shrunk_to_fit_the_card`: four columns of long text at `max_w = 300`, every row measures no more than 300 in the code face, and every row's column starts agree
- [x] 2.4 Test `a_ragged_row_is_padded_to_the_widest`: a body row with fewer cells than the header keeps the header's column starts

## 3. Wrap inside a cell

- [x] 3.1 Each cell wrapped to `width[i] * unit` in the code face; a logical row is as tall as its tallest cell; shorter cells pad with empty lines
- [x] 3.2 Test `a_cells_text_wraps_and_the_row_grows`: a long cell in a 2-column table produces two physical rows and the neighbouring cell's second line is padding
- [x] 3.3 Test `cjk_in_a_cell_keeps_the_column_starts`: a CJK cell still leaves column 2 starting at the same offset on every row

## 4. Alignment

- [x] 4.1 `Right` pads on the left, `Center` splits with the extra unit on the right, `None`/`Left` pad on the right
- [x] 4.2 Test `alignment_pads_the_short_side`: three columns `:---`, `:---:`, `---:` with texts of different lengths, each cell's own text starts where its alignment says

## 5. The rest of the panel

- [x] 5.1 Every table row carries one `Style::Code` span over the whole line, so the orthographic face is the code face; `indent` and `quote` follow the block's nesting
- [x] 5.2 Test `a_table_row_copies_with_its_padding`: the selection over a table row copies the drawn string, alignment spaces included
- [x] 5.3 Update `marks_outside_the_subset_stay_readable`: the table case becomes a task list and raw HTML; a table no longer stays as written

## 6. Docs and gate

- [x] 6.1 `README`, `docs/aside.1`, `CLAUDE.md` known limits, `docs/live-checklist.md`
- [x] 6.2 `--preview` scene shows a table
- [x] 6.3 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`, `cargo audit`
