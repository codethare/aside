//! Answers as markdown: one parse per delta into the panel's own rows.
//!
//! The parser is `pulldown-cmark` with every optional extension off, so a table or
//! a task list stays the characters the model sent. The renderer is this file: it
//! turns events into [`Line`]s, and a `Line` holds only the characters a reader
//! sees, so a selection and a copy address exactly those.
//!
//! Layout is incremental at block granularity. A block that ends at a blank line
//! outside a fence can never be changed by text that arrives later, so it is
//! rendered once and kept; only the open tail is rendered again per delta. That is
//! the shape the plain wrapper had, and it keeps a long answer from being re-laid
//! out from the top on every chunk.

use std::ops::Range;

use pulldown_cmark::{Alignment, Event, Options, Parser, Tag, TagEnd};

use crate::font::PanelFont;
use crate::layout::{self, Line, Span, Style};
use crate::render;

/// One answer's rows, with the part that can still change rendered separately.
#[derive(Debug, Default, Clone)]
pub struct Layout {
    /// Rows of the blocks that are settled.
    done: Vec<Line>,
    /// Byte offset in the answer where the open tail begins.
    from: usize,
    /// Rows of the open tail, thrown away and rebuilt as text arrives.
    tail: Vec<Line>,
}

impl Layout {
    /// Lay the answer out: only the part that can still change is rendered again.
    pub fn update(&mut self, text: &str, font: &PanelFont, max_w: f32) {
        let cut = boundary(text, self.from);
        if cut > self.from {
            let before = !self.done.is_empty();
            let more = render(&text[self.from..cut], font, max_w, before);
            self.done.extend(more);
            self.from = cut;
        }
        self.tail = render(&text[self.from..], font, max_w, !self.done.is_empty());
    }

    /// One more row outside the answer's text: the mark that says the rest of a
    /// too-long answer was dropped.
    pub fn mark(&mut self, text: &str, font: &PanelFont, max_w: f32) {
        self.tail
            .extend(render(text, font, max_w, !self.tail.is_empty()));
    }

    pub fn rows(&self) -> impl Iterator<Item = &Line> {
        self.done.iter().chain(self.tail.iter())
    }

    pub fn len(&self) -> usize {
        self.done.len() + self.tail.len()
    }

    /// Give the room back: a retired turn is kept for the request, not for the box.
    pub fn clear(&mut self) {
        self.done = Vec::new();
        self.tail = Vec::new();
        self.from = 0;
    }
}

/// Where the settled part ends: the last blank line at or after `from` that is not
/// inside a fenced block. Everything before it is a closed sequence of blocks;
/// everything after still depends on what has not arrived.
fn boundary(text: &str, from: usize) -> usize {
    let mut at = from;
    let mut fence: Option<String> = None;
    let mut cut = from;
    for line in text[from..].split_inclusive('\n') {
        let trimmed = line.trim();
        match (&fence, fence_marker(trimmed)) {
            // A fence closes only on a marker of its own kind and length.
            (Some(open), Some(marker)) if marker.len() >= open.len() => fence = None,
            (Some(_), _) => {}
            (None, Some(marker)) => fence = Some(marker),
            (None, None) if trimmed.is_empty() => cut = at + line.len(),
            (None, None) => {}
        }
        at += line.len();
    }
    cut
}

/// The fence a line opens with (` ``` ` or `~~~`, three or more), if any.
fn fence_marker(trimmed: &str) -> Option<String> {
    let marker = trimmed.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let run = trimmed.chars().take_while(|c| *c == marker).count();
    (run >= 3).then(|| marker.to_string().repeat(run))
}

/// Blocks in, rows out.
fn render(text: &str, font: &PanelFont, max_w: f32, before: bool) -> Vec<Line> {
    let mut r = Renderer {
        font,
        pad: crate::PAD * layout::scale_of(font),
        max_w,
        out: Vec::new(),
        text: String::new(),
        spans: Vec::new(),
        marker: None,
        heading: false,
        code: false,
        inline: Inline::default(),
        quote: 0,
        lists: Vec::new(),
        table: None,
        before,
    };
    for event in Parser::new_ext(text, Options::ENABLE_TABLES) {
        r.event(event);
    }
    r.flush();
    r.out
}

/// The narrowest a column may be shrunk to, in space units.
const MIN_UNITS: usize = 3;

/// A table as the parser hands it over: the column alignments, the header row, the
/// body rows, and the cells of the row being read.
#[derive(Default)]
struct Table {
    align: Vec<Alignment>,
    head: Vec<Cell>,
    rows: Vec<Vec<Cell>>,
    row: Vec<Cell>,
    in_head: bool,
}

/// One cell's characters, before it is padded into a column. Its marks are dropped on
/// purpose: a cell drawn in its own face would leave its column, so the table's ink is
/// the cell's ink until one span can hold both.
#[derive(Default)]
struct Cell {
    text: String,
}

#[derive(Default, Clone, Copy)]
struct Inline {
    strong: bool,
    em: bool,
    link: bool,
}

/// The paragraph being collected, and the blocks around it.
struct Renderer<'a> {
    font: &'a PanelFont,
    /// One nesting step in buffer px.
    pad: u32,
    max_w: f32,
    out: Vec<Line>,
    /// The characters of the block being collected.
    text: String,
    spans: Vec<Span>,
    /// The bullet or number the next list item starts with.
    marker: Option<String>,
    heading: bool,
    code: bool,
    inline: Inline,
    /// Quote nesting: any depth puts a rail beside the rows.
    quote: usize,
    /// One entry per open list: `None` for bullets, `Some(next)` for numbers.
    lists: Vec<Option<u64>>,
    /// The table being read, while one is open.
    table: Option<Table>,
    /// Whether the answer already has rows above these: a slice of an answer is laid
    /// out on its own, and the separation at the seam must still be there.
    before: bool,
}

impl Renderer<'_> {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => {
                let style = self.style();
                push(&mut self.text, &mut self.spans, &text, style);
            }
            // A code span's spacing is exact: a copy must reproduce it.
            Event::Code(text) => push(&mut self.text, &mut self.spans, &text, Style::Code),
            Event::SoftBreak => push(&mut self.text, &mut self.spans, " ", Style::Plain),
            Event::HardBreak => self.flush(),
            // Not in the subset, so it is not drawn as anything else: a rule is one
            // empty row, which is the separation it stands for.
            Event::Rule => {
                self.flush();
                self.out.push(Line::blank());
            }
            // A stray tag from a model is better read than swallowed.
            Event::Html(html) | Event::InlineHtml(html) => {
                push(&mut self.text, &mut self.spans, &html, Style::Plain)
            }
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {}
            Tag::Heading { .. } => {
                self.flush();
                self.heading = true;
            }
            Tag::CodeBlock(_) => {
                self.flush();
                self.code = true;
            }
            Tag::List(start) => {
                self.flush();
                self.lists.push(start);
            }
            Tag::Item => {
                self.flush();
                self.marker = Some(match self.lists.last_mut() {
                    Some(Some(next)) => {
                        let text = format!("{next}.");
                        *next += 1;
                        text
                    }
                    _ => "\u{2022}".to_string(),
                });
            }
            Tag::BlockQuote(_) => {
                self.flush();
                self.quote += 1;
            }
            Tag::Emphasis => self.inline.em = true,
            Tag::Strong => self.inline.strong = true,
            Tag::Link { .. } => self.inline.link = true,
            Tag::Image { .. } => {}
            Tag::Table(align) => {
                self.flush();
                self.table = Some(Table {
                    align,
                    ..Table::default()
                });
            }
            Tag::TableHead => {
                if let Some(table) = self.table.as_mut() {
                    table.in_head = true;
                }
            }
            Tag::TableRow => {
                if let Some(table) = self.table.as_mut() {
                    table.row.clear();
                }
            }
            Tag::TableCell => {
                // A cell's inline content collects in the same buffer a paragraph uses.
                self.text.clear();
                self.spans.clear();
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Item => self.flush(),
            TagEnd::Heading(_) => {
                self.flush();
                self.heading = false;
            }
            TagEnd::CodeBlock => {
                self.flush_code();
                self.code = false;
            }
            TagEnd::List(_) => {
                self.flush();
                self.lists.pop();
            }
            TagEnd::BlockQuote(_) => {
                self.flush();
                self.quote = self.quote.saturating_sub(1);
            }
            TagEnd::TableCell => {
                let cell = Cell {
                    text: std::mem::take(&mut self.text)
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                };
                self.spans.clear();
                if let Some(table) = self.table.as_mut() {
                    table.row.push(cell);
                }
            }
            TagEnd::TableHead => {
                if let Some(table) = self.table.as_mut() {
                    table.head = std::mem::take(&mut table.row);
                    table.in_head = false;
                }
            }
            TagEnd::TableRow => {
                if let Some(table) = self.table.as_mut() {
                    table.rows.push(std::mem::take(&mut table.row));
                }
            }
            TagEnd::Table => self.flush_table(),
            TagEnd::Emphasis => self.inline.em = false,
            TagEnd::Strong => self.inline.strong = false,
            TagEnd::Link => self.inline.link = false,
            _ => {}
        }
    }

    /// The inline style the flags add up to, a heading's own strength included.
    fn style(&self) -> Style {
        if self.code {
            Style::Code
        } else if self.heading || self.inline.strong {
            Style::Strong
        } else if self.inline.em {
            Style::Em
        } else if self.inline.link {
            Style::Link
        } else {
            Style::Plain
        }
    }

    /// Left offset of this block's text: one `pad` per list or quote around it.
    fn indent(&self) -> u32 {
        self.pad * (self.lists.len() + self.quote) as u32
    }

    fn flush(&mut self) {
        if self.code {
            return; // closed by `flush_code`, once the whole block has arrived
        }
        if self.text.trim().is_empty() {
            self.text.clear();
            self.spans.clear();
            return;
        }
        // A block is one empty row away from the block before it: the grid never
        // moves, and a reader sees where one thing ended.
        self.separate();
        self.paragraph();
    }

    /// One empty row, unless this block continues the one before it: a list's items
    /// sit tight, and so does a quote's.
    fn separate(&mut self) {
        if (self.before || !self.out.is_empty()) && self.lists.is_empty() {
            self.out.push(Line::blank());
        }
    }

    fn paragraph(&mut self) {
        let text = std::mem::take(&mut self.text).trim().to_string();
        let spans = std::mem::take(&mut self.spans);
        if text.is_empty() {
            self.marker = None;
            return;
        }
        let indent = self.indent();
        let quote = self.quote > 0;
        // A list item's bullet sits one step left of its text and the wrapped rows
        // hang under the text: that is the whole of the hanging indent.
        let base = match &self.marker {
            Some(_) => indent.saturating_sub(self.pad),
            None => indent,
        };
        let prefix = self.marker.take().map(|marker| format!("{marker} "));
        let text_indent = match &prefix {
            Some(prefix) => base + render::advance(self.font, prefix) as u32,
            None => base,
        };
        let breadth = self.max_w - text_indent as f32;
        // A paragph reads the code face only when it holds a code span.
        let mono = spans
            .iter()
            .any(|s| s.style == Style::Code)
            .then(|| self.font.mono())
            .flatten();
        let mut rows = wrap(
            &text,
            &spans,
            text_indent,
            breadth,
            self.font,
            mono.as_deref(),
            Style::Plain,
        );
        if let (Some(prefix), Some(first)) = (prefix, rows.first_mut()) {
            // The bullet becomes part of the first row's own text, so a copy of the
            // line carries it and a selection can cover it.
            let shift = prefix.len();
            for span in &mut first.spans {
                span.range.start += shift;
                span.range.end += shift;
            }
            first.text.insert_str(0, &prefix);
            first.indent = base;
        }
        for row in &mut rows {
            row.quote = quote;
        }
        self.out.extend(rows);
    }

    /// A table, as rows of padded columns. Everything is drawn in the code face: a
    /// column lines up only when a padding space is the same width as the characters
    /// beside it.
    fn flush_table(&mut self) {
        let Some(table) = self.table.take() else {
            return;
        };
        // A table is drawn in the code face: a column lines up only when a padding
        // space is as wide as the characters beside it.
        let mono = self.font.mono();
        let face = mono.as_deref().unwrap_or(self.font);
        let unit = table_unit(face);
        let columns = table
            .align
            .len()
            .max(table.head.len())
            .max(table.rows.iter().map(Vec::len).max().unwrap_or(0));
        if columns == 0 {
            return;
        }
        let mut width = vec![MIN_UNITS; columns];
        for row in std::iter::once(&table.head).chain(table.rows.iter()) {
            for (i, cell) in row.iter().enumerate() {
                width[i] = width[i].max(cell_units(face, &cell.text, unit));
            }
        }
        let indent = self.indent();
        let quote = self.quote > 0;
        // Fit the card: the widest column loses a unit at a time, so a wide table is
        // narrowed rather than cut, and no column is squeezed below a few characters.
        // One space after every column's text: it separates a full cell from the next
        // column's, and the last one is trimmed away.
        let avail = ((self.max_w - indent as f32) / unit as f32)
            .floor()
            .max(1.0) as usize;
        let mut total: usize = width.iter().sum::<usize>() + columns - 1;
        while total > avail {
            let widest = width
                .iter()
                .enumerate()
                .filter(|(_, w)| **w > MIN_UNITS)
                .max_by_key(|(_, w)| **w)
                .map(|(i, _)| i);
            let Some(i) = widest else { break };
            width[i] -= 1;
            total -= 1;
        }
        let align = &table.align;
        let mut out: Vec<Line> = Vec::new();
        let row_of = |cells: &[Cell], out: &mut Vec<Line>| {
            // Each cell wraps inside its own column; the row is as tall as the tallest
            // of them, and a shorter cell pads with empty lines so the columns below
            // stay under themselves.
            let wrapped: Vec<Vec<String>> = (0..columns)
                .map(|i| {
                    let text = cells.get(i).map_or("", |c| c.text.as_str());
                    let breadth = width[i] as f32 * unit as f32;
                    wrap(
                        text,
                        &[],
                        0,
                        breadth,
                        self.font,
                        mono.as_deref(),
                        Style::Code,
                    )
                    .into_iter()
                    .map(|l| l.text)
                    .collect()
                })
                .collect();
            let tall = wrapped.iter().map(Vec::len).max().unwrap_or(0).max(1);
            for line in 0..tall {
                let mut text = String::new();
                for (i, w) in width.iter().enumerate() {
                    let piece = wrapped[i].get(line).map_or("", |s| s.as_str());
                    let used = cell_units(face, piece, unit).min(*w);
                    // The colons in the rule are alignment: the short side carries the
                    // padding, and a centre's odd unit goes to the right.
                    let spare = w - used;
                    let (left, right) = match align.get(i) {
                        Some(Alignment::Right) => (spare, 0),
                        Some(Alignment::Center) => (spare / 2, spare - spare / 2),
                        _ => (0, spare),
                    };
                    for _ in 0..left {
                        text.push(' ');
                    }
                    text.push_str(piece);
                    for _ in 0..right {
                        text.push(' ');
                    }
                    text.push(' ');
                }
                out.push(table_line(text.trim_end(), indent, quote));
            }
        };
        row_of(&table.head, &mut out);
        // The rule marks the header: with one style per span, "monospace and bolder"
        // cannot be said, so the boundary is drawn instead.
        let total: usize = width.iter().sum::<usize>() + columns - 1;
        let mut rule = String::new();
        let span = total as f32 * unit as f32;
        // Measured in the face the row is *drawn* in, which is the text face: the span
        // below is `Style::Plain`, and `draw_row` picks the face from the style.
        while render::advance(self.font, &rule) < span && rule.len() < 4096 {
            rule.push('\u{2500}');
        }
        out.push(Line {
            text: rule,
            spans: vec![Span {
                range: 0..0,
                style: Style::Plain,
            }],
            indent,
            code: false,
            quote,
        });
        if let Some(last) = out.last_mut() {
            last.spans[0].range = 0..last.text.len();
            last.spans.retain(|s| s.range.end > 0);
        }
        for row in &table.rows {
            row_of(row, &mut out);
        }
        self.separate();
        self.out.extend(out);
    }

    /// A fenced block: each source line is its own row, measured in the code face and
    /// wrapped inside the block.
    fn flush_code(&mut self) {
        self.separate();
        let text = std::mem::take(&mut self.text);
        let indent = self.indent() + self.pad;
        let mono = self.font.mono();
        let mut rows = 0;
        for line in text.lines() {
            let breadth = self.max_w - indent as f32;
            let mut line_rows = wrap(
                line,
                &[],
                indent,
                breadth,
                self.font,
                mono.as_deref(),
                Style::Code,
            );
            for row in &mut line_rows {
                row.code = true;
                row.quote = self.quote > 0;
            }
            if !line_rows.is_empty() {
                rows += 1;
            }
            self.out.extend(line_rows);
        }
        if rows == 0 {
            let mut row = Line::blank();
            row.code = true;
            row.indent = indent;
            self.out.push(row);
        }
        self.spans.clear();
    }
}

/// One space in the code face: the unit every column's padding is counted in.
fn table_unit(face: &PanelFont) -> u32 {
    (render::advance(face, " ").ceil() as u32).max(1)
}

/// How many space units `text` occupies in the code face.
fn cell_units(face: &PanelFont, text: &str, unit: u32) -> usize {
    ((render::advance(face, text) / unit as f32).ceil() as usize).max(1)
}

/// A table row as a line: the whole of it is code, padding included, because that is
/// what puts the columns in their places.
fn table_line(text: &str, indent: u32, quote: bool) -> Line {
    Line {
        text: text.to_string(),
        spans: vec![Span {
            range: 0..text.len(),
            style: Style::Code,
        }],
        indent,
        code: false,
        quote,
    }
}

/// Add `text` in `style`, merging into the span before it when they touch.
fn push(text: &mut String, spans: &mut Vec<Span>, what: &str, style: Style) {
    let start = text.len();
    text.push_str(what);
    if text.len() > start && style != Style::Plain {
        if let Some(last) = spans.last_mut()
            && last.style == style
            && last.range.end == start
        {
            last.range.end = text.len();
            return;
        }
        spans.push(Span {
            range: start..text.len(),
            style,
        });
    }
}

/// Wrap one block's text into rows: `indent` is off the width and drawn as a left
/// offset, and each span is clipped to the row it lands on.
fn wrap(
    text: &str,
    spans: &[Span],
    indent: u32,
    breadth: f32,
    font: &PanelFont,
    mono: Option<&PanelFont>,
    plain_style: Style,
) -> Vec<Line> {
    let limit = breadth.max(1.0);
    let adv = |byte: usize, c: char| {
        let style = match layout::style_at(spans, byte) {
            Style::Plain => plain_style,
            other => other,
        };
        match style {
            Style::Code => layout::char_advance(mono.unwrap_or(font), c),
            _ => layout::char_advance(font, c),
        }
    };
    let mut out = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let rest = &text[start..];
        let cut = match layout::break_tail(rest, &adv, limit) {
            Some(at) => at,
            None => match layout::hard_break(rest) {
                Some(at) => at,
                None => rest.len(),
            },
        };
        let cut = cut.max(1).min(rest.len());
        let row_text = rest[..cut].trim_end();
        let kept = row_text.len();
        if kept > 0 {
            let mut row = Line {
                text: row_text.to_string(),
                spans: clip_spans(spans, start, start + kept),
                indent,
                code: plain_style == Style::Code,
                quote: false,
            };
            if row.spans.is_empty() && plain_style != Style::Plain {
                row.spans.push(Span {
                    range: 0..kept,
                    style: plain_style,
                });
            }
            out.push(row);
        }
        start += cut;
        if text[start..].starts_with('\n') {
            start += 1;
        }
    }
    out
}

/// The spans of `[from, to)`, rebased onto the row that holds them.
fn clip_spans(spans: &[Span], from: usize, to: usize) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    for span in spans {
        let start = span.range.start.max(from);
        let end = span.range.end.min(to);
        if start < end {
            let range: Range<usize> = (start - from)..(end - from);
            if let Some(last) = out.last_mut()
                && last.style == span.style
                && last.range.end == range.start
            {
                last.range.end = range.end;
                continue;
            }
            out.push(Span {
                range,
                style: span.style,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::char_advance;

    fn font() -> PanelFont {
        PanelFont::load(crate::font::SIZE).expect("system font available")
    }

    /// An answer as the panel would draw it, row by row.
    fn rows(text: &str, f: &PanelFont, max_w: f32) -> Vec<Line> {
        render(text, f, max_w, false)
    }

    fn texts(rows: &[Line]) -> Vec<&str> {
        rows.iter().map(|r| r.text.as_str()).collect()
    }

    #[test]
    fn latin_wraps_at_spaces_and_never_overflows() {
        let f = font();
        let max_w = 120.0;
        let text = "the quick brown fox jumps over the lazy dog and keeps running";
        let lines = rows(text, &f, max_w);
        assert!(lines.len() > 1, "this must wrap");
        for row in &lines {
            assert!(
                render::measure(&f, &row.text, f32::MAX) <= max_w,
                "row overflows the panel: {:?}",
                row.text
            );
        }
        // No word is lost or split mid-word.
        let joined = texts(&lines).join(" ");
        for word in text.split(' ') {
            assert!(joined.contains(word), "lost {word:?}");
        }
    }

    #[test]
    fn a_long_word_is_broken_rather_than_clipped() {
        let f = font();
        let lines = rows(&"W".repeat(80), &f, 100.0);
        assert!(lines.len() > 1);
        for row in &lines {
            assert!(render::measure(&f, &row.text, f32::MAX) <= 100.0);
        }
        assert_eq!(texts(&lines).concat(), "W".repeat(80));
    }

    #[test]
    fn cjk_wraps_per_character() {
        let f = font();
        let text = "这是一个很长的问题，需要折行显示，因为面板没有这么宽";
        let lines = rows(text, &f, 130.0);
        assert!(lines.len() > 1, "CJK must wrap without spaces");
        for row in &lines {
            assert!(render::measure(&f, &row.text, f32::MAX) <= 130.0);
        }
        assert_eq!(texts(&lines).concat(), text, "no character may be lost");
    }

    #[test]
    fn mixed_latin_and_cjk_wraps_at_both_kinds_of_boundary() {
        let f = font();
        let text = "error EAGAIN 表示资源暂时不可用，稍后重试即可";
        let lines = rows(text, &f, 150.0);
        assert!(lines.len() > 1);
        assert_eq!(texts(&lines).concat(), text);
        for row in &lines {
            assert!(
                render::measure(&f, &row.text, f32::MAX) <= 150.0,
                "row {:?}",
                row.text
            );
        }
    }

    #[test]
    fn a_character_wider_than_the_panel_breaks_on_a_boundary() {
        let f = font();
        // A CJK character is about 20 px wide, so a 12 px panel cannot hold one: the
        // fallback break must still land on a character boundary.
        for text in ["\u{4e2d}", "\u{4e2d}\u{6587}\u{5b57}", "a\u{4e2d}"] {
            let lines = rows(text, &f, 12.0);
            assert_eq!(texts(&lines).concat(), text, "no character may be lost");
        }
    }

    #[test]
    fn a_newline_inside_a_paragraph_is_a_soft_break() {
        let f = font();
        // Markdown's soft break is a space; an empty line is what ends a paragraph.
        assert_eq!(texts(&rows("first\nsecond", &f, 500.0)), ["first second"]);
        // Two paragraphs are two blocks, one empty row apart.
        assert_eq!(
            texts(&rows("first\n\nsecond", &f, 500.0)),
            ["first", "", "second"]
        );
    }

    /// The markup is never in the text: a copy takes what the reader sees, and the
    /// spans are what carry the style.
    #[test]
    fn markup_is_never_in_the_row_text() {
        let f = font();
        let lines = rows(
            "a **bold** and *slanted* and `code` [link](https://x)",
            &f,
            900.0,
        );
        let text = texts(&lines).join(" ");
        assert_eq!(text, "a bold and slanted and code link");
        assert!(!text.contains('*') && !text.contains('`') && !text.contains(']'));
        let styles: Vec<(Style, &str)> = lines
            .iter()
            .flat_map(|row| {
                row.spans
                    .iter()
                    .map(move |span| (span.style, &row.text[span.range.clone()]))
            })
            .collect();
        assert!(styles.contains(&(Style::Strong, "bold")), "{styles:?}");
        assert!(styles.contains(&(Style::Em, "slanted")), "{styles:?}");
        assert!(styles.contains(&(Style::Code, "code")), "{styles:?}");
        assert!(styles.contains(&(Style::Link, "link")), "{styles:?}");
    }

    #[test]
    fn a_fenced_block_is_its_own_rows_in_the_code_face() {
        let f = font();
        let lines = rows("before\n\n```rust\nlet x = 1;\n````\nafter", &f, 900.0);
        let texts = texts(&lines);
        assert!(texts.contains(&"let x = 1;"), "{texts:?}");
        assert!(!texts.iter().any(|t| t.starts_with("```")), "{texts:?}");
        let code: Vec<&Line> = lines.iter().filter(|l| l.code).collect();
        assert_eq!(code.len(), 1, "the fence's own rows only");
        assert_eq!(code[0].spans[0].style, Style::Code);
        assert!(code[0].indent > 0, "code is set in from the card");
        // and it is measured in the code face, which is what lines the columns up
        assert!(f.mono().is_some(), "this system has a monospace face");
    }

    #[test]
    fn a_list_hangs_its_wrapped_rows_under_its_own_text() {
        let f = font();
        let long = "an item long enough to wrap onto a second row of the panel";
        let lines = rows(&format!("- {long}"), &f, 200.0);
        assert!(lines.len() > 1, "the item wraps: {:?}", texts(&lines));
        assert!(
            lines[0].text.starts_with("\u{2022} "),
            "{:?}",
            lines[0].text
        );
        // The first row starts one step left of the text and carries the bullet, so
        // its text begins where the wrapped rows' own indent puts them.
        let bullet = render::advance(&f, "\u{2022} ") as u32;
        assert_eq!(
            lines[1].indent,
            lines[0].indent + bullet,
            "a wrapped row lines up under the text, not under the bullet"
        );
    }

    #[test]
    fn a_heading_gets_one_empty_row_above_it_and_bold_text() {
        let f = font();
        let lines = rows("intro\n\n## Title\n\nafter", &f, 900.0);
        let i = texts(&lines)
            .iter()
            .position(|t| *t == "Title")
            .expect("the heading's row");
        assert!(i > 0);
        assert_eq!(lines[i - 1].text, "", "one empty row above the heading");
        assert_eq!(lines[i].spans[0].style, Style::Strong);
        // At the very top there is no empty row: the card's padding is the separation.
        assert_eq!(texts(&rows("## Title", &f, 900.0))[0], "Title");
    }

    #[test]
    fn a_quote_is_indented_and_marked() {
        let f = font();
        let lines = rows("> quoted words", &f, 900.0);
        assert_eq!(texts(&lines), ["quoted words"]);
        assert!(lines[0].quote, "the rail has to know it is a quote");
        assert!(lines[0].indent > 0);
    }

    /// Outside the subset nothing is lost: a task list and a raw tag are drawn as the
    /// characters the model sent. (A table is inside the subset now; see
    /// `a_table_is_columns_of_padded_cells`.)
    #[test]
    fn marks_outside_the_subset_stay_readable() {
        let f = font();
        let task = texts(&rows("- [ ] not done\n- [x] done", &f, 900.0)).join(" ");
        assert!(task.contains("[ ] not done"), "{task:?}");
        assert!(task.contains("[x] done"), "{task:?}");
        let html = texts(&rows("<b>raw</b>", &f, 900.0)).join(" ");
        assert_eq!(html, "<b>raw</b>");
    }

    /// A table wider than the card has its widest columns narrowed until it fits,
    /// and its columns still start in the same places.
    #[test]
    fn a_wide_table_is_shrunk_to_fit_the_card() {
        let f = font();
        let Some(mono) = f.mono() else {
            return; // no monospace face: the table is drawn in the text face
        };
        let table = "| first column of words | second column of words | third column of words |\
                     fourth column of words |\n|---|---|---|---|\n\
                     | alpha beta gamma | delta epsilon zeta | eta theta iota | kappa lambda mu |";
        let max_w = 300.0;
        let lines = rows(table, &f, max_w);
        assert!(lines.len() >= 3, "{:?}", texts(&lines));
        for line in &lines {
            let wide = render::measure(&mono, &line.text, f32::MAX);
            assert!(
                wide <= max_w + 1.0,
                "a row is {wide} wide in a {max_w} card: {:?}",
                line.text
            );
        }
        // Every row of a table starts its columns in the same place, header included.
        let header = lines[0].text.as_str();
        let body = lines
            .iter()
            .skip(1)
            .find(|l| l.text.starts_with("alpha"))
            .expect("the body row");
        for (a, b) in [("second", "delta"), ("third", "eta"), ("fourth", "kappa")] {
            assert_eq!(
                header.find(a),
                body.text.find(b),
                "column {a} moved: {:?} vs {:?}",
                header,
                body.text
            );
        }
    }

    /// A cell too wide for its column wraps inside it, and the row grows to the
    /// tallest cell: the neighbour's second line is padding.
    #[test]
    fn a_cells_text_wraps_and_the_row_grows() {
        let f = font();
        let table = "| a | b |\n|---|---|\n| a cell long enough to need two rows of its \
                     own in the column | short |";
        let lines = rows(table, &f, 260.0);
        let texts = texts(&lines);
        let body: Vec<&&str> = texts
            .iter()
            .filter(|t| t.starts_with('a') && !t.trim_end().ends_with('|'))
            .collect();
        assert!(body.len() >= 2, "the long cell wrapped: {texts:?}");
        // The short cell's column holds its text on the first row and padding after.
        let first = texts
            .iter()
            .position(|t| t.contains("short"))
            .expect("the row holding the short cell");
        let next = texts.get(first + 1).copied().unwrap_or("");
        assert!(
            next.trim_end().len() < texts[first].trim_end().len(),
            "the neighbour pads rather than repeating: {texts:?}"
        );
    }

    /// A CJK cell is measured, not counted: column two still starts where the header
    /// put it, even though its neighbour's characters are not one space wide.
    #[test]
    fn cjk_in_a_cell_keeps_the_column_starts() {
        let f = font();
        let lines = rows("| 名称 | 说明 |\n|---|---|\n| 网络 | 连接超时 |", &f, 600.0);
        let texts = texts(&lines);
        let header = texts
            .iter()
            .find(|t| t.contains("名称") && t.contains("说明"))
            .unwrap_or_else(|| panic!("the header row holds both cells: {texts:?}"));
        let body = texts
            .iter()
            .find(|t| t.contains("网络") && t.contains("连接"))
            .unwrap_or_else(|| panic!("the body row holds both cells: {texts:?}"));
        let column_of =
            |row: &str, needle: &str| render::advance(&f, &row[..row.find(needle).expect("cell")]);
        assert!(
            (column_of(header, "说明") - column_of(body, "连接")).abs() < 1.0,
            "column two moved between rows: {header:?} {body:?}"
        );
    }

    /// The colons in a table's rule are alignment: a column's own text sits left, in
    /// the middle, or at its right edge.
    #[test]
    fn alignment_pads_the_short_side() {
        let f = font();
        let table = "| l | c | r |\n| :--- | :---: | ---: |\n\
                     | abcdef | ghijkl | mnopqr |";
        let lines = rows(table, &f, 600.0);
        let texts = texts(&lines);
        let header = texts
            .iter()
            .find(|t| t.contains('l') && t.contains('r'))
            .expect("header");
        // Six units a column, one space between: columns start at 0, 7 and 14.
        assert_eq!(header.find('l'), Some(0), "left stays left: {header:?}");
        assert_eq!(
            header.find('c'),
            Some(9),
            "centred in six units: {header:?}"
        );
        assert_eq!(header.find('r'), Some(19), "right to its edge: {header:?}");
    }

    /// The rule is drawn in the dim ink's own face, so it must be *measured* in that
    /// face: a dash counted in the code face comes out a few pixels short of the
    /// table it is under.
    #[test]
    fn the_rule_is_as_wide_as_the_table_it_underlines() {
        let f = font();
        let Some(mono) = f.mono() else {
            return; // no monospace face: no unit to measure against
        };
        let lines = rows("| alpha | beta |\n|---|---|\n| 1 | 2 |", &f, 900.0);
        let rule = lines
            .iter()
            .find(|l| l.text.starts_with('\u{2500}'))
            .expect("the rule row");
        assert_eq!(rule.spans[0].style, Style::Plain, "dim ink");
        let drawn = render::advance(&f, &rule.text);
        let table = render::advance(&mono, &lines[0].text);
        let dash = render::advance(&f, "\u{2500}").max(1.0);
        assert!(
            (drawn - table).abs() < dash,
            "the rule is {drawn} under a {table} table: {:?} / {:?}",
            rule.text,
            lines[0].text
        );
    }

    /// A table takes the nesting it sits in: indented inside a list or a quote, and
    /// carrying the rail when it is quoted.
    #[test]
    fn a_table_steps_in_with_its_block() {
        let f = font();
        let plain = rows("| a | b |\n|---|---|\n| 1 | 2 |", &f, 900.0);
        assert_eq!(plain[0].indent, 0, "at the card's edge");
        let quoted = rows("> | a | b |\n> |---|---|\n> | 1 | 2 |", &f, 900.0);
        let header = quoted
            .iter()
            .find(|l| l.text.contains('a') && l.text.contains('b'))
            .expect("the quoted table's header");
        assert!(header.indent > 0, "a quote steps the table in");
        assert!(header.quote, "and puts its rail beside it");
    }

    /// A table that cannot be narrowed enough is drawn and clipped, never reflowed or
    /// dropped: it stops at the narrowest column and the painter cuts the rest.
    #[test]
    fn a_table_that_cannot_shrink_is_still_a_table() {
        let f = font();
        let header: String = (0..12).map(|i| format!(" | c{i}")).collect();
        let rule: String = (0..12).map(|_| " | ---").collect();
        let body: String = (0..12).map(|i| format!(" | v{i}")).collect();
        let table = format!("{header}\n{rule}\n{body}");
        let lines = rows(&table, &f, 60.0);
        assert!(
            lines.len() >= 3,
            "the table is still drawn: {:?}",
            texts(&lines)
        );
        // Every row still starts its cells in the same places, even though the table
        // is wider than the card: the painter clips it, the renderer does not reflow it.
        let starts = |row: &str| -> Vec<usize> {
            let mut out = Vec::new();
            let mut after_space = true;
            for (i, c) in row.char_indices() {
                if c != ' ' && after_space {
                    out.push(i);
                }
                after_space = c == ' ';
            }
            out
        };
        let want = starts(&lines[0].text);
        assert_eq!(want.len(), 12, "twelve columns: {:?}", lines[0].text);
        // The rule between them is one run of dashes, not a row of cells.
        for line in lines.iter().filter(|l| !l.text.starts_with('\u{2500}')) {
            assert_eq!(starts(&line.text), want, "a row lost its columns: {line:?}");
        }
    }

    /// A copy takes a row's own string, so the spaces the columns are made of are
    /// part of what is copied: trimming them would collapse the table.
    #[test]
    fn a_table_row_carries_its_padding_as_text() {
        let f = font();
        let lines = rows("| a | b |\n|---|---|\n| long cell | 2 |", &f, 600.0);
        let header = lines
            .iter()
            .find(|l| l.text.starts_with('a'))
            .expect("the header row");
        assert!(
            header.text.contains("  "),
            "the gap after the cell is the column's padding: {:?}",
            header.text
        );
        assert_eq!(header.spans.len(), 1, "one span over the whole row");
        assert_eq!(header.spans[0].style, Style::Code, "in the code face");
        assert_eq!(
            header.spans[0].range,
            0..header.text.len(),
            "padding included"
        );
    }

    /// A row with fewer cells than the header still lines up with it.
    #[test]
    fn a_ragged_row_is_padded_to_the_widest() {
        let f = font();
        let lines = rows("| a | b | c |\n|---|---|---|\n| 1 | 2 |", &f, 900.0);
        let texts = texts(&lines);
        let header = texts.iter().find(|t| t.starts_with('a')).expect("header");
        let body = texts.iter().find(|t| t.starts_with('1')).expect("body");
        assert_eq!(
            header.find('b'),
            body.find('2'),
            "the second column is where the header put it: {header:?} {body:?}"
        );
    }

    /// The document that panicked `into_offset_iter()` in 0.13.4: the pin exists so
    /// this cannot come back, and this is where a loosened pin fails.
    #[test]
    fn the_document_that_panicked_the_parser_is_laid_out() {
        let f = font();
        for text in [
            "- [o]:u\n    \t",
            "- [o]:u\n    \t\n\nmore",
            "[]]]",
            "[[a|]] ",
        ] {
            let _ = rows(text, &f, 300.0);
        }
    }

    /// The settled part is only ever the text before a blank line outside a fence, and
    /// laying an answer out in pieces gives the same rows as laying it out once.
    #[test]
    fn laying_out_piece_by_piece_matches_laying_out_once() {
        let f = font();
        let text = "streaming answers arrive in pieces\n\n- and a **list**\n- with items\n\n\
                    ```\ncode in a fence\n```\n\nand a last paragraph";
        let once: Vec<String> = texts(&rows(text, &f, 240.0))
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut layout = Layout::default();
        let mut arrived = String::new();
        for piece in text.as_bytes().chunks(7) {
            arrived.push_str(&String::from_utf8_lossy(piece));
            layout.update(&arrived, &f, 240.0);
        }
        let pieces: Vec<String> = layout.rows().map(|l| l.text.clone()).collect();
        assert_eq!(pieces, once);
    }

    /// A blank line inside a fence is not a block boundary: a code block stays one
    /// block however it was chunked, or half of it would be laid out twice.
    #[test]
    fn a_blank_line_inside_a_fence_is_not_a_boundary() {
        // An open fence settles nothing: half a code block laid out twice would be
        // two blocks.
        assert_eq!(boundary("```\nfirst\n\nsecond\n```\n", 0), 0);
        // Once the fence closes, the blank line after it is a boundary.
        let closed = "```\nfirst\n\nsecond\n```\n\nnext\n";
        assert_eq!(boundary(closed, 0), closed.len() - "next\n".len());
        // A blank line outside it is one.
        assert_eq!(boundary("para\n\n```\nfenced\n", 0), 6);
    }

    #[test]
    fn the_code_face_is_only_read_when_a_fence_needs_it() {
        let f = font();
        let _ = rows("plain prose, no code", &f, 300.0);
        assert!(
            f.code_face_unbuilt(),
            "a plain answer must not read a second font"
        );
        let _ = rows("```\nx\n```", &f, 300.0);
        assert!(!f.code_face_unbuilt());
    }

    #[test]
    fn a_code_row_is_measured_in_the_code_face() {
        let f = font();
        let mono = f.mono();
        let Some(mono) = mono.as_deref() else {
            return; // no monospace face on this system
        };
        // Eight characters of code: one advance each, in the code face.
        let row = rows("```\niiiiiiii\n```", &f, 2000.0)
            .into_iter()
            .find(|l| l.code)
            .expect("the code row");
        let want: f32 = row.text.chars().map(|c| char_advance(mono, c)).sum();
        let got: f32 = row.text.chars().map(|c| char_advance(&f, c)).sum();
        assert!(mono.size == f.size);
        assert!(
            (want - got).abs() > 0.01,
            "the code face must measure differently: mono {want} text {got}"
        );
    }
}
