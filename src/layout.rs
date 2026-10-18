//! Text layout for the panel: wrapping a growing answer, the scrollback
//! viewport, and the input line's caret.
//!
//! Wrapping is incremental on purpose. An answer arrives as a stream of deltas,
//! and re-wrapping the whole transcript for each one is O(n²) and visibly janks;
//! `Lines` therefore keeps only the row currently being filled plus the rows
//! already closed, and a delta touches the tail alone.

use crate::font::PanelFont;
use crate::render::{self, Subpixel};

/// Most transcript rows the panel ever draws. The buffer is bounded, so the
/// viewport is too; the frame cost is per visible row.
pub const MAX_ROWS: usize = 64;

/// A break is allowed between these and their neighbours: CJK ideographs, kana
/// and full-width forms break per character, everything else at spaces.
///
/// This is the Latin + CJK subset of UAX #14, not the full table: the panel
/// renders no shaped scripts (see README), so the rules it would need are the
/// two that matter. `shortcut: no hyphenation/kinsoku tables; add
/// unicode-linebreak if a script beyond Latin and CJK needs breaking.`
fn breaks_after(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x11FF      // Hangul jamo
        | 0x2E80..=0x303E    // CJK radicals, punctuation, kana
        | 0x3041..=0x33FF    // kana, CJK compatibility
        | 0x3400..=0x4DBF    // CJK ext A
        | 0x4E00..=0x9FFF    // CJK unified
        | 0xA000..=0xA4CF    // Yi
        | 0xAC00..=0xD7A3    // Hangul syllables
        | 0xF900..=0xFAFF    // CJK compatibility ideographs
        | 0xFE30..=0xFE4F    // CJK compatibility forms
        | 0xFF00..=0xFF60    // full-width forms
        | 0xFFE0..=0xFFE6
        | 0x20000..=0x3FFFD  // CJK ext B and beyond
    )
}

fn is_space(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// One wrapped display row of a growing block of text.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Lines {
    /// Rows that will not change as more text arrives.
    rows: Vec<String>,
    /// The row being filled: it can still grow, so it is not a row yet.
    tail: String,
}

impl Lines {
    #[cfg(test)]
    pub fn rows(&self) -> &[String] {
        &self.rows
    }

    /// Rows plus the partial tail, which is what the panel draws.
    pub fn visible_rows(&self) -> impl Iterator<Item = &str> {
        let tail = (!self.tail.is_empty()).then_some(self.tail.as_str());
        self.rows.iter().map(String::as_str).chain(tail)
    }

    pub fn total_rows(&self) -> usize {
        self.rows.len() + usize::from(!self.tail.is_empty())
    }

    /// Append a delta and close every row it makes completable, so the cost is
    /// proportional to the delta and not to the answer so far.
    pub fn push(&mut self, delta: &str, font: &PanelFont, max_w: f32) {
        self.tail.push_str(delta);
        loop {
            if let Some(at) = break_tail(&self.tail, font, max_w) {
                let rest = self.tail.split_off(at);
                let row = std::mem::replace(&mut self.tail, rest);
                self.rows.push(row.trim_end().to_string());
            } else if let Some(at) = hard_break(&self.tail) {
                let rest = self.tail.split_off(at + 1);
                let row = std::mem::replace(&mut self.tail, rest);
                self.rows.push(row.trim_end().to_string());
            } else {
                break;
            }
        }
    }

    /// Close the row still being filled: the answer has ended, or a newline did.
    pub fn finish(&mut self) {
        if !self.tail.is_empty() {
            let row = std::mem::take(&mut self.tail);
            self.rows.push(row.trim_end().to_string());
        }
    }

    /// Everything as rows, for a caller that has the whole text at once.
    pub fn wrap(text: &str, font: &PanelFont, max_w: f32) -> Lines {
        let mut lines = Lines::default();
        lines.push(text, font, max_w);
        lines.finish();
        lines
    }
}

/// Byte index of the first newline, if the tail holds one.
fn hard_break(s: &str) -> Option<usize> {
    s.find('\n')
}

/// Byte index to split `s` at so the first part fits `max_w` and ends at a break
/// opportunity, or `None` when the whole tail still fits (so it can still grow).
fn break_tail(s: &str, font: &PanelFont, max_w: f32) -> Option<usize> {
    if s.is_empty() || render::measure(font, s, f32::MAX) <= max_w {
        return None;
    }
    // Walk characters, remembering the last break opportunity that still fits.
    let mut width = 0.0;
    let mut last: Option<usize> = None;
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let adv = char_advance(font, c);
        if width + adv > max_w {
            // This character would overflow. Break at the last opportunity; if
            // there was none, break here so an over-wide character (or a very
            // narrow panel) still makes progress.
            return Some(last.unwrap_or(i).max(1).min(s.len()));
        }
        width += adv;
        let next = chars.peek().map(|(_, c)| *c);
        let breakable = match next {
            None => false,
            Some(n) => is_space(c) || breaks_after(c) || breaks_after(n),
        };
        if breakable {
            // A break after a space drops the space from the rendered row.
            last = Some(i + c.len_utf8());
        }
    }
    None
}

/// Advance width of one character in the chain.
pub fn char_advance(font: &PanelFont, c: char) -> f32 {
    // No face carries it: advance by the widest thing we can measure so the
    // layout cannot loop forever on a character nothing can draw.
    font.glyph(c)
        .map(|(_, _, advance)| advance)
        .unwrap_or(font.size / 2.0)
}

/// A scrollback window over the transcript's rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    /// First visible row.
    pub top: usize,
    /// Rows the window can show, never more than `MAX_ROWS`.
    pub height: usize,
    /// Whether the view is pinned to the newest row.
    pub following: bool,
}

impl Viewport {
    pub fn new(height: usize) -> Viewport {
        Viewport {
            top: 0,
            height: height.clamp(1, MAX_ROWS),
            following: true,
        }
    }

    /// Rows currently on screen.
    pub fn visible(&self, total: usize) -> std::ops::Range<usize> {
        let top = self.top.min(total.saturating_sub(1));
        let end = (top + self.height).min(total);
        top..end
    }

    /// Keep `top` inside the transcript, and stay at the tail while following.
    pub fn clamp(&mut self, total: usize) {
        if self.following {
            self.top = total.saturating_sub(self.height);
        }
        self.top = self.top.min(total.saturating_sub(1));
    }

    /// Re-pin to the tail, as sending a question does.
    pub fn follow(&mut self, total: usize) {
        self.following = true;
        self.clamp(total);
    }

    /// Scroll so `row` is inside the window (the highlight must stay visible).
    /// Moving the highlight back to the newest row resumes following.
    pub fn ensure_visible(&mut self, row: usize, total: usize) {
        if row < self.top {
            self.top = row;
            self.following = false;
        } else if row >= self.top + self.height {
            self.top = row + 1 - self.height;
            self.following = self.top + self.height >= total;
        }
        self.top = self.top.min(total.saturating_sub(1));
    }
}

/// The input line: one logical line, soft-wrapped by the painter, with a caret
/// held as a byte index on a character boundary.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Input {
    text: String,
    caret: usize,
}

impl Input {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }

    pub fn take(&mut self) -> String {
        self.caret = 0;
        std::mem::take(&mut self.text)
    }

    pub fn insert(&mut self, s: &str) {
        // Newlines are not a key here, but a pasted question can carry them.
        self.text.insert_str(self.caret, s);
        self.caret += s.len();
    }

    /// Delete the character before the caret (or the selection, which this
    /// panel does not have: one logical line, no selection).
    pub fn backspace(&mut self) {
        if let Some(c) = self.text[..self.caret].chars().next_back() {
            self.caret -= c.len_utf8();
            self.text.remove(self.caret);
        }
    }

    pub fn delete(&mut self) {
        if let Some(c) = self.text[self.caret..].chars().next() {
            self.text.remove(self.caret);
            let _ = c;
        }
    }

    pub fn left(&mut self) {
        if let Some(c) = self.text[..self.caret].chars().next_back() {
            self.caret -= c.len_utf8();
        }
    }

    pub fn right(&mut self) {
        if let Some(c) = self.text[self.caret..].chars().next() {
            self.caret += c.len_utf8();
        }
    }

    pub fn home(&mut self) {
        self.caret = 0;
    }

    pub fn end(&mut self) {
        self.caret = self.text.len();
    }

    /// Move by one word, the way `Ctrl+Left`/`Ctrl+Right` do in a text field:
    /// skip whitespace, then the word itself.
    pub fn word_left(&mut self) {
        let before = &self.text[..self.caret];
        let trimmed = before.trim_end();
        let cut = match trimmed.rfind(|c: char| c.is_whitespace() || breaks_after(c)) {
            Some(i) => i + trimmed[i..].chars().next().map_or(0, char::len_utf8),
            None => 0,
        };
        self.caret = cut;
    }

    pub fn word_right(&mut self) {
        let after = &self.text[self.caret..];
        let skip = after
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(after.len());
        let rest = &after[skip..];
        let end = rest
            .find(|c: char| c.is_whitespace() || breaks_after(c))
            .unwrap_or(rest.len());
        self.caret += skip + end;
    }
}

/// Colour of the 4 px rail beside the input: the working indicator. It changes
/// a colour and no geometry, so a stream starting or ending cannot move a row.
pub fn rail_color(streaming: bool) -> crate::render::Bgra {
    if streaming {
        render::BG_SEL
    } else {
        render::LABEL_ACCENT
    }
}

/// Antialiasing for text at a given scale: subpixel only at 1×, where it is
/// legible; above that it costs 35–55 % more for colour fringes nobody wants.
pub fn text_aa(scale: u32) -> Subpixel {
    if scale <= 1 {
        Subpixel::Rgb
    } else {
        Subpixel::Gray
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> PanelFont {
        PanelFont::load(crate::font::SIZE).expect("system font available")
    }

    #[test]
    fn latin_wraps_at_spaces_and_never_overflows() {
        let f = font();
        let max_w = 120.0;
        let text = "the quick brown fox jumps over the lazy dog and keeps running";
        let lines = Lines::wrap(text, &f, max_w);
        assert!(lines.rows().len() > 1, "this must wrap");
        for row in lines.rows() {
            assert!(
                render::measure(&f, row, f32::MAX) <= max_w,
                "row overflows the panel: {row:?}"
            );
        }
        // No word is lost or split mid-word.
        let joined = lines.rows().join(" ");
        for word in text.split(' ') {
            assert!(joined.contains(word), "lost {word:?}");
        }
    }

    #[test]
    fn a_long_word_is_broken_rather_than_clipped() {
        let f = font();
        let lines = Lines::wrap(&"W".repeat(80), &f, 100.0);
        assert!(lines.rows().len() > 1);
        for row in lines.rows() {
            assert!(render::measure(&f, row, f32::MAX) <= 100.0);
        }
        assert_eq!(lines.rows().concat(), "W".repeat(80));
    }

    #[test]
    fn cjk_wraps_per_character() {
        let f = font();
        let text = "这是一个很长的问题，需要折行显示，因为面板没有这么宽";
        let lines = Lines::wrap(text, &f, 130.0);
        assert!(lines.rows().len() > 1, "CJK must wrap without spaces");
        for row in lines.rows() {
            assert!(render::measure(&f, row, f32::MAX) <= 130.0);
        }
        assert_eq!(lines.rows().concat(), text, "no character may be lost");
    }

    #[test]
    fn mixed_latin_and_cjk_wraps_at_both_kinds_of_boundary() {
        let f = font();
        let text = "error EAGAIN 表示资源暂时不可用，稍后重试即可";
        let lines = Lines::wrap(text, &f, 150.0);
        assert!(lines.rows().len() > 1);
        assert_eq!(lines.rows().concat(), text);
        for row in lines.rows() {
            assert!(render::measure(&f, row, f32::MAX) <= 150.0, "row {row:?}");
        }
    }

    #[test]
    fn wrapping_is_incremental_over_deltas() {
        let f = font();
        // The same text wrapped as one block and as 200 deltas must agree: this
        // is what makes a streaming answer cheap to lay out.
        let text: String = "streaming answers arrive in pieces ".repeat(20);
        let once = Lines::wrap(&text, &f, 200.0);
        let mut incremental = Lines::default();
        for piece in text.as_bytes().chunks(7) {
            incremental.push(&String::from_utf8_lossy(piece), &f, 200.0);
        }
        incremental.finish();
        assert_eq!(incremental.rows(), once.rows());
    }

    #[test]
    fn a_newline_forces_a_row_end() {
        let f = font();
        let lines = Lines::wrap("first\nsecond", &f, 500.0);
        assert_eq!(lines.rows(), ["first", "second"]);
    }

    #[test]
    fn the_tail_grows_until_it_must_break() {
        let f = font();
        let mut lines = Lines::default();
        lines.push("hello ", &f, 500.0);
        assert_eq!(lines.rows().len(), 0, "a row that still fits is not closed");
        assert_eq!(lines.total_rows(), 1);
        lines.push("there", &f, 500.0);
        assert_eq!(lines.rows().len(), 0);
        lines.finish();
        assert_eq!(lines.rows(), ["hello there"]);
    }

    #[test]
    fn the_viewport_never_holds_more_than_the_cap() {
        let mut v = Viewport::new(1000);
        assert_eq!(v.height, MAX_ROWS);
        v.ensure_visible(10, 500);
        assert!(v.visible(500).len() <= MAX_ROWS);
        v.follow(500);
        assert_eq!(v.visible(500), 500 - MAX_ROWS..500);
    }

    #[test]
    fn new_rows_do_not_drag_a_reader_who_scrolled_up() {
        let mut v = Viewport::new(10);
        v.clamp(100);
        assert!(v.following);
        // A reader who moves the highlight to an older row keeps that row.
        v.ensure_visible(50, 100);
        assert!(!v.following);
        assert_eq!(v.top, 50, "the highlighted row is at the top of the window");
        // Rows arriving must not pull the view back to the tail.
        v.clamp(140);
        assert_eq!(v.top, 50);
        // Coming back to the newest row resumes following.
        v.ensure_visible(139, 140);
        assert!(v.following);
        assert_eq!(v.top, 130);
    }

    #[test]
    fn ensure_visible_brings_the_highlight_into_the_window() {
        let mut v = Viewport::new(4);
        v.follow(40);
        assert_eq!(v.top, 36);
        v.ensure_visible(10, 40);
        assert!(v.visible(40).contains(&10));
        v.ensure_visible(39, 40);
        assert!(v.visible(40).contains(&39));
    }

    #[test]
    fn rail_colour_is_the_only_thing_that_changes_while_streaming() {
        assert_eq!(rail_color(false), render::LABEL_ACCENT);
        assert_eq!(rail_color(true), render::BG_SEL);
    }

    #[test]
    fn subpixel_aa_only_at_scale_one() {
        assert_eq!(text_aa(1), Subpixel::Rgb);
        assert_eq!(text_aa(2), Subpixel::Gray);
        assert_eq!(text_aa(3), Subpixel::Gray);
    }

    #[test]
    fn typing_edits_at_the_caret() {
        let mut i = Input::default();
        i.insert("hello");
        i.left();
        i.left();
        i.insert("z");
        assert_eq!(i.text(), "helzlo");
        i.backspace();
        assert_eq!(i.text(), "hello");
        i.delete();
        assert_eq!(i.text(), "helo");
        i.home();
        i.delete();
        assert_eq!(i.text(), "elo");
        i.end();
        i.insert("!");
        assert_eq!(i.text(), "elo!");
    }

    #[test]
    fn cjk_is_deleted_by_character_not_by_byte() {
        let mut i = Input::default();
        i.insert("折行");
        i.backspace();
        assert_eq!(i.text(), "折");
        i.end();
        i.insert("行");
        i.left();
        i.insert("test");
        assert_eq!(i.text(), "折test行");
    }

    #[test]
    fn word_jumps_skip_whitespace_then_the_word() {
        let mut i = Input::default();
        i.insert("what does 折行 mean");
        i.home();
        i.word_right();
        assert_eq!(&i.text()[..i.caret()], "what");
        i.word_right();
        assert_eq!(&i.text()[..i.caret()], "what does");
        i.end();
        i.word_left();
        assert_eq!(&i.text()[i.caret()..], "mean");
    }

    #[test]
    fn an_empty_question_is_blank() {
        let mut i = Input::default();
        i.insert("   \n ");
        assert!(i.is_blank());
        assert_eq!(i.take(), "   \n ");
        assert!(i.text().is_empty() && i.caret() == 0);
    }
}
