//! The conversation: turns in order, the open answer streaming into the last
//! entry, and what may be sent back to the provider.

use crate::font::PanelFont;
use crate::layout::{Blocks, Lines};
use crate::provider::{Role, Turn};

/// Most of one answer the panel keeps. A provider can stream far more than a
/// panel can show, and the whole thing is laid out in memory.
pub const REPLY_CAP: usize = 64 * 1024;

/// What an entry is, which decides how it is drawn and whether it can be sent
/// back to the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A question the user typed.
    Question,
    /// An answer.
    Answer,
    /// A failure, shown in the transcript rather than swallowed.
    Error,
    /// Something the panel says about itself: cancelled, truncated.
    Note,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub kind: Kind,
    /// The turn's text as it arrived. Kept so the rows can be laid out again for
    /// a new panel width without guessing the wrapping back from the rows.
    pub text: String,
    /// That text, wrapped for the current width.
    pub lines: Lines,
    /// False while an answer is streaming, and after it was cancelled or cut off
    /// by a dropped connection.
    pub complete: bool,
    /// The answer hit `REPLY_CAP` and the rest was dropped.
    pub truncated: bool,
    /// Buffer-pixel top of this entry's block, gaps and padding included.
    pub y: u32,
    /// Its height. Cached because the viewport places rows by pixel and walking
    /// the transcript to find one would cost more than the frame.
    pub h: u32,
}

#[derive(Debug, Default)]
pub struct Transcript {
    entries: Vec<Entry>,
    /// Index of the answer being streamed into, if any.
    open: Option<usize>,
    /// Height of the blocks on screen, gaps included. Empty blocks take no room.
    height: u32,
}

/// One row as the painter needs it: the text, what it belongs to, and where it
/// goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row<'a> {
    pub text: &'a str,
    pub kind: Kind,
    /// Buffer-pixel top of this row.
    pub y: u32,
    /// The block this row belongs to: its top and height, card padding included.
    pub block: (u32, u32),
}

impl Transcript {
    /// Test-only: the panel asks the request, not the transcript, whether a
    /// stream is open.
    #[cfg(test)]
    pub fn is_streaming(&self) -> bool {
        self.open.is_some()
    }

    #[cfg(test)]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    fn push(&mut self, kind: Kind, complete: bool, blocks: Blocks) -> usize {
        // Blocks are appended, never inserted, so the next one starts after the
        // last inked pixel plus one gap.
        let y = if self.height == 0 {
            0
        } else {
            self.height + blocks.gap
        };
        self.entries.push(Entry {
            kind,
            text: String::new(),
            lines: Lines::default(),
            complete,
            truncated: false,
            y,
            h: 0,
        });
        self.entries.len() - 1
    }

    /// Re-measure the tail entry and with it the transcript's whole height: only
    /// the newest block can grow, because nothing is ever inserted before it.
    fn settle(&mut self, blocks: Blocks) {
        if let Some(entry) = self.entries.last_mut() {
            entry.h = entry.block_h(blocks);
            // An empty tail sits where the next block will start but must not
            // stretch the transcript over its own gap.
            if entry.h > 0 {
                self.height = entry.y + entry.h;
            }
        }
    }

    /// Index of the first entry that is on screen: the turn the last question
    /// opened. Everything before it is kept for the request and laid out never.
    fn current_start(&self) -> usize {
        self.entries
            .iter()
            .rposition(|e| e.kind == Kind::Question)
            .map_or(0, |i| i + 1)
    }

    /// Drop the layout of a turn that is no longer on screen, keeping its text:
    /// the provider still gets it, the box does not.
    fn retire(entries: &mut [Entry], start: usize) {
        for entry in &mut entries[..start] {
            entry.lines = Lines::default();
            entry.h = 0;
            entry.y = 0;
        }
    }

    /// Retire everything before the question just pushed, and restart the block
    /// chain: the previous turn is no longer on screen.
    fn retire_current(&mut self) {
        let start = self.entries.len();
        Self::retire(&mut self.entries, start);
        self.height = 0;
    }

    /// Lay every entry out again: a resize changes the width, and the wrap is a
    /// property of the width, not of the text.
    pub fn rewrap(&mut self, font: &PanelFont, max_w: f32) {
        let blocks = Blocks::new(font);
        let start = self.current_start();
        Self::retire(&mut self.entries, start);
        let mut y = 0;
        let mut inked = false;
        for entry in &mut self.entries[start..] {
            if entry.kind == Kind::Question {
                // Not drawn, so never laid out: no rows, no height, no gap.
                entry.lines = Lines::default();
                entry.h = 0;
                entry.y = y;
                continue;
            }
            entry.lines = Lines::wrap(&entry.text, font, max_w);
            entry.h = entry.block_h(blocks);
            if entry.h == 0 {
                entry.y = y;
                continue;
            }
            if inked {
                y += blocks.gap;
            }
            entry.y = y;
            y += entry.h;
            inked = true;
        }
        self.height = y;
    }

    /// Record a question and open an answer for it, so a delta has somewhere to
    /// go. The question is kept for the next request and laid out never: only the
    /// reply is on screen.
    pub fn ask(&mut self, question: &str, font: &PanelFont) {
        let blocks = Blocks::new(font);
        // A previous answer that never finished is closed before a new question:
        // two open answers would have no defined meaning, and closing it first is
        // what gives it its last row before the answer is placed under it.
        self.close_open(blocks);
        // Only the turn being asked is on screen: the previous one keeps its text
        // for the request and gives up its layout, so the block chain restarts.
        self.retire_current();
        let user = self.push(Kind::Question, true, blocks);
        self.entries[user].text.push_str(question);
        self.open = Some(self.push(Kind::Answer, false, blocks));
    }

    /// Append streamed text to the open answer.
    pub fn delta(&mut self, text: &str, font: &PanelFont, max_w: f32) {
        let Some(i) = self.open else { return };
        let blocks = Blocks::new(font);
        let entry = &mut self.entries[i];
        if entry.truncated {
            return; // the cap is reached and already marked
        }
        let room = REPLY_CAP.saturating_sub(entry.lines_text_len());
        let (take, cut) = split_at_char_boundary(text, room);
        if !take.is_empty() {
            entry.text.push_str(take);
            entry.lines.push(take, font, max_w);
        }
        if cut {
            entry.truncated = true;
            // One visible mark that the rest was dropped, rather than a silent cut.
            entry.lines.push(" \u{2026}", font, max_w);
            entry.lines.finish();
        }
        self.settle(blocks);
    }

    /// The answer ended normally.
    pub fn finish_answer(&mut self, font: &PanelFont) {
        if let Some(i) = self.open.take() {
            self.entries[i].lines.finish();
            self.entries[i].complete = true;
            self.settle(Blocks::new(font));
        }
    }

    /// The answer was cancelled or the stream was cut: it stays on screen, and
    /// it is not sent back to the provider as if it were an answer.
    pub fn cancel(&mut self, note: &str, font: &PanelFont, max_w: f32) {
        let blocks = Blocks::new(font);
        if let Some(i) = self.open.take() {
            self.entries[i].lines.finish();
            self.settle(blocks);
            self.add_note(note, font, max_w);
        }
    }

    /// A failure: its own entry, distinct from answer text.
    pub fn fail(&mut self, message: &str, font: &PanelFont, max_w: f32) {
        let blocks = Blocks::new(font);
        self.close_open(blocks);
        let i = self.push(Kind::Error, true, blocks);
        self.entries[i].text.push_str(message);
        self.entries[i].lines.push(message, font, max_w);
        self.entries[i].lines.finish();
        self.settle(blocks);
    }

    /// Something the panel says about itself.
    pub fn add_note(&mut self, text: &str, font: &PanelFont, max_w: f32) {
        let blocks = Blocks::new(font);
        let i = self.push(Kind::Note, true, blocks);
        self.entries[i].text.push_str(text);
        self.entries[i].lines.push(text, font, max_w);
        self.entries[i].lines.finish();
        self.settle(blocks);
    }

    fn close_open(&mut self, blocks: Blocks) {
        if let Some(i) = self.open.take() {
            self.entries[i].lines.finish();
            self.settle(blocks);
        }
    }

    /// Rows in display order, with the styling the painter needs. The current
    /// turn's, because that is what is on screen.
    pub fn rows(&self, blocks: Blocks) -> Vec<Row<'_>> {
        let mut out = Vec::new();
        for entry in &self.entries[self.current_start()..] {
            for (i, row) in entry.lines.visible_rows().enumerate() {
                out.push(Row {
                    text: row,
                    kind: entry.kind,
                    y: entry.y + i as u32 * blocks.row_h,
                    block: (entry.y, entry.h),
                });
            }
        }
        out
    }

    /// Buffer-pixel top of the `index`th row, which is what the viewport scrolls
    /// to when the highlight moves.
    pub fn row_y(&self, index: usize, blocks: Blocks) -> Option<u32> {
        let mut left = index;
        for entry in &self.entries[self.current_start()..] {
            let rows = entry.lines.total_rows();
            if left < rows {
                return Some(entry.y + left as u32 * blocks.row_h);
            }
            left -= rows;
        }
        None
    }

    /// Which row a transcript-local `y` falls on: the walk `row_y` does, in the
    /// other direction. A pointer lands between two rows' ink (in a gap, or in a
    /// block's padding) as easily as on one, so a y inside a block's own span finds
    /// the row nearest it.
    pub fn row_at(&self, y: u32, blocks: Blocks) -> Option<usize> {
        let mut index = 0;
        for entry in &self.entries[self.current_start()..] {
            let rows = entry.lines.total_rows();
            if rows == 0 {
                continue;
            }
            let top = entry.y;
            let bottom = entry.y + entry.h;
            if y >= top && y < bottom {
                let row = ((y - top) / blocks.row_h).min(rows as u32 - 1);
                return Some(index + row as usize);
            }
            index += rows;
        }
        None
    }

    /// The next word's start after `(row, ch)`, crossing lines: a word starts after
    /// whitespace, and a CJK character starts one of its own, which is where the
    /// wrapper breaks too. Clamps to the end of the transcript when there is none.
    pub fn word_forward(&self, (row, ch): (usize, usize), blocks: Blocks) -> (usize, usize) {
        let rows = self.rows(blocks);
        let from = (row.min(rows.len().saturating_sub(1)), ch);
        for (index, row) in rows.iter().enumerate() {
            for (at, c) in row.text.char_indices() {
                if (index, at) > from && starts_word(c, prev_of(row.text, at)) {
                    return (index, at);
                }
            }
        }
        let last = rows.len().saturating_sub(1);
        (last, rows.get(last).map_or(0, |r| r.text.len()))
    }

    /// The start of the word before `(row, ch)`, crossing lines. Stops at the first
    /// word of the transcript.
    pub fn word_back(&self, (row, ch): (usize, usize), blocks: Blocks) -> (usize, usize) {
        let rows = self.rows(blocks);
        let from = (row.min(rows.len().saturating_sub(1)), ch);
        let mut found = (0, 0);
        for (index, row) in rows.iter().enumerate() {
            for (at, c) in row.text.char_indices() {
                if (index, at) < from && starts_word(c, prev_of(row.text, at)) {
                    found = (index, at);
                }
            }
        }
        found
    }

    /// Height of everything the transcript draws, gaps included.
    pub fn total_height(&self) -> u32 {
        self.height
    }

    pub fn total_rows(&self) -> usize {
        self.entries[self.current_start()..]
            .iter()
            .map(|e| e.lines.total_rows())
            .sum()
    }

    /// What the next request carries: this run's questions and the answers that
    /// finished. An answer that was cancelled or cut off is left out — sending
    /// half an answer back would present it as something the model said.
    pub fn request_turns(&self) -> Vec<Turn> {
        let mut turns = Vec::new();
        for entry in &self.entries {
            let role = match entry.kind {
                Kind::Question => Role::User,
                Kind::Answer if entry.complete => Role::Assistant,
                _ => continue,
            };
            let text = entry.text.clone();
            if !text.trim().is_empty() {
                turns.push(Turn { role, text });
            }
        }
        turns
    }
}

/// The character before byte `at`, if there is one.
fn prev_of(text: &str, at: usize) -> Option<char> {
    text[..at].chars().next_back()
}

/// Whether a word begins at `c`: after whitespace, and at every CJK character,
/// which the wrapper breaks after and which no dictionary would join into one word.
fn starts_word(c: char, prev: Option<char>) -> bool {
    if c.is_whitespace() {
        return false;
    }
    match prev {
        None => true,
        Some(prev) => {
            prev.is_whitespace()
                || crate::layout::breaks_after(prev)
                || crate::layout::breaks_after(c)
        }
    }
}

/// Split `s` at or before `at`, never inside a character. Returns the prefix and
/// whether anything was dropped.
fn split_at_char_boundary(s: &str, at: usize) -> (&str, bool) {
    if at >= s.len() {
        return (s, false);
    }
    let mut end = at;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    (&s[..end], true)
}

impl Entry {
    /// Height of this entry's block: its text rows and nothing else. An entry with
    /// no text takes no room at all, so an answer that has not started yet cannot
    /// leave a gap under its question.
    fn block_h(&self, blocks: Blocks) -> u32 {
        self.lines.total_rows() as u32 * blocks.row_h
    }

    /// Text length of this entry's rows, which is what the cap counts. The rows
    /// are the same characters as `text` plus the row breaks, so `text` is the
    /// measure.
    fn lines_text_len(&self) -> usize {
        self.text.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> PanelFont {
        PanelFont::load(crate::font::SIZE).expect("system font available")
    }

    fn text_of(t: &Transcript, f: &PanelFont) -> Vec<String> {
        t.rows(Blocks::new(f))
            .iter()
            .map(|r| r.text.to_string())
            .collect()
    }

    /// The answers are the only rows on screen: a question is kept for the next
    /// request and drawn nowhere.
    #[test]
    fn the_answers_are_the_only_rows() {
        let f = font();
        let mut t = Transcript::default();
        t.ask("why", &f);
        t.delta("because", &f, 400.0);
        t.finish_answer(&f);
        assert_eq!(text_of(&t, &f), ["because"], "the first answer, alone");
        t.ask("and?", &f);
        t.delta("this too", &f, 400.0);
        t.finish_answer(&f);
        // The box holds the current turn only: the first answer is kept for the
        // request and is not on screen.
        assert_eq!(text_of(&t, &f), ["this too"]);
        assert_eq!(t.total_height(), f.row_h, "one line of room, not two");
        assert!(!t.is_streaming());
        // Nothing on screen is a question, and the request still has them.
        assert!(
            t.rows(Blocks::new(&f))
                .iter()
                .all(|r| r.kind != Kind::Question)
        );
        let turns = t.request_turns();
        assert_eq!(turns.len(), 4, "two questions, two answers");
        assert_eq!(turns[0].text, "why");
        assert_eq!(turns[2].text, "and?");
    }

    #[test]
    fn the_request_carries_the_finished_turns_in_order() {
        let f = font();
        let mut t = Transcript::default();
        t.ask("first", &f);
        t.delta("answer one", &f, 400.0);
        t.finish_answer(&f);
        t.ask("second", &f);
        let turns = t.request_turns();
        assert_eq!(turns.len(), 3, "the open question is included");
        assert_eq!(turns[0].role, Role::User);
        assert_eq!(turns[0].text, "first");
        assert_eq!(turns[1].role, Role::Assistant);
        assert_eq!(turns[1].text, "answer one");
        assert_eq!(turns[2].text, "second");
    }

    #[test]
    fn a_cancelled_answer_is_shown_but_never_sent_back() {
        let f = font();
        let mut t = Transcript::default();
        t.ask("first", &f);
        t.delta("half an ans", &f, 400.0);
        t.cancel("cancelled", &f, 400.0);
        // While it is the current turn, the half answer and its note are on screen.
        let rows = text_of(&t, &f);
        assert!(rows.contains(&"half an ans".to_string()), "still visible");
        assert!(rows.contains(&"cancelled".to_string()));
        t.ask("second", &f);
        t.delta("full answer", &f, 400.0);
        t.finish_answer(&f);
        // The new turn is what the box holds now.
        assert_eq!(text_of(&t, &f), ["full answer"]);
        let turns = t.request_turns();
        assert!(
            turns
                .iter()
                .all(|t| t.role != Role::Assistant || t.text == "full answer"),
            "no incomplete answer is sent: {turns:?}"
        );
        assert_eq!(turns.len(), 3, "first, second, and the finished answer");
    }

    #[test]
    fn an_answer_that_never_ended_is_not_sent_after_the_stream_dies() {
        let f = font();
        let mut t = Transcript::default();
        t.ask("q", &f);
        t.delta("partial", &f, 400.0);
        t.fail("connection lost", &f, 400.0);
        let turns = t.request_turns();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].text, "q");
        assert!(text_of(&t, &f).iter().any(|r| r == "connection lost"));
    }

    #[test]
    fn a_failure_is_its_own_entry_kind() {
        let f = font();
        let mut t = Transcript::default();
        t.fail("401 unauthorized: COMMAND_CODE_API_KEY", &f, 400.0);
        let rows = t.rows(Blocks::new(&f));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, Kind::Error);
        assert_eq!(
            t.total_height(),
            f.row_h,
            "a failure is one row of the replies card"
        );
    }

    /// Blocks are placed by their real heights, so a card is taller than a row
    /// and every block is separated by one gap.
    #[test]
    fn blocks_are_separated_by_a_gap_and_never_overlap() {
        let f = font();
        let blocks = Blocks::new(&f);
        let mut t = Transcript::default();
        t.ask("why", &f);
        t.delta(
            "because the sky scatters short wavelengths more than long ones",
            &f,
            60.0,
        );
        t.finish_answer(&f);
        // A note belongs to this turn, so the box holds two blocks: the answer,
        // then one gap, then the note.
        t.add_note("and that is that", &f, 60.0);

        let rows = t.rows(blocks);
        assert!(rows.len() > 3, "the answer must wrap to several rows");
        // Every row is inside its own block, and the blocks are contiguous.
        let mut end = 0;
        for row in &rows {
            assert!(
                row.y >= row.block.0 && row.y + blocks.row_h <= row.block.0 + row.block.1,
                "row outside its block"
            );
            end = row.block.0 + row.block.1;
        }
        assert_eq!(end, t.total_height(), "the last block ends the transcript");
        // Two blocks: the note starts one gap below the answer.
        let mut cards = rows.iter().map(|r| r.block).collect::<Vec<_>>();
        cards.dedup();
        assert_eq!(cards.len(), 2, "the answer and the note");
        assert_eq!(
            cards[1].0,
            cards[0].0 + cards[0].1 + blocks.gap,
            "one gap between the blocks"
        );
        for (_, h) in cards {
            assert_eq!(h % blocks.row_h, 0, "a block is whole rows tall");
        }
    }

    /// An answer that has not started takes no room: the card appears with its
    /// first row, not before it.
    #[test]
    fn an_empty_answer_occupies_nothing() {
        let f = font();
        let mut t = Transcript::default();
        t.ask("why", &f);
        assert_eq!(t.total_height(), 0, "a question alone takes no room");
        assert_eq!(t.total_rows(), 0, "and no rows to draw");
        t.delta("x", &f, 400.0);
        assert_eq!(
            t.total_height(),
            Blocks::new(&f).row_h,
            "the card appeared with its first row"
        );
    }

    /// The whole point of keeping the older turns: the provider sees them even
    /// though the box does not, and the box starts at zero for each new turn.
    #[test]
    fn a_retired_turn_keeps_its_text_and_loses_its_room() {
        let f = font();
        let blocks = Blocks::new(&f);
        let mut t = Transcript::default();
        t.ask("first", &f);
        t.delta(
            "an answer long enough to wrap at a narrow width, twice over",
            &f,
            80.0,
        );
        t.finish_answer(&f);
        assert!(t.total_rows() > 2, "the first turn wrapped");
        t.ask("second", &f);
        // Nothing of the first turn is laid out now, and the new answer starts at 0.
        assert_eq!(t.total_rows(), 0, "the new turn has no text yet");
        assert_eq!(t.total_height(), 0);
        t.delta("second answer", &f, 80.0);
        t.finish_answer(&f);
        let rows = t.rows(blocks);
        assert_eq!(
            rows[0].y, 0,
            "the current turn starts at the top of the box"
        );
        assert_eq!(rows[0].text, "second", "the current turn's own text");
        // The request still has every turn, in order.
        let turns = t.request_turns();
        assert_eq!(turns.len(), 4, "two questions, two answers");
        assert_eq!(turns[0].text, "first");
        assert!(turns[1].text.starts_with("an answer long enough"));
        assert_eq!(turns[2].text, "second");
        assert_eq!(turns[3].text, "second answer");
        // A resize rewraps the current turn only, and it still starts at 0.
        t.rewrap(&f, 40.0);
        assert_eq!(t.rows(blocks)[0].y, 0);
        assert_eq!(t.request_turns().len(), 4, "nothing was lost");
    }

    /// The pointer's way in: a y inside a block names that block's row, and a y in
    /// the gap between blocks names nothing rather than the block above it.
    #[test]
    fn a_pointer_position_names_a_row() {
        let f = font();
        let blocks = Blocks::new(&f);
        let mut t = Transcript::default();
        t.ask("why", &f);
        t.delta(
            "a first answer long enough to wrap at a narrow width",
            &f,
            80.0,
        );
        t.finish_answer(&f);
        t.add_note("and a note", &f, 80.0);
        for index in 0..t.total_rows() {
            let y = t.row_y(index, blocks).expect("every row has a place");
            assert_eq!(t.row_at(y, blocks), Some(index), "row {index} at {y}");
            assert_eq!(
                t.row_at(y + blocks.row_h - 1, blocks),
                Some(index),
                "the row is found by any pixel of it"
            );
        }
        // The gap between two blocks belongs to neither.
        let rows = t.rows(blocks);
        let mut cards: Vec<(u32, u32)> = rows.iter().map(|r| r.block).collect();
        cards.dedup();
        assert!(cards.len() > 1, "two blocks to have a gap between");
        let gap = cards[0].0 + cards[0].1 + blocks.gap / 2;
        assert_eq!(t.row_at(gap, blocks), None, "a gap names no row");
        assert_eq!(t.row_at(u32::MAX, blocks), None, "past the end");
    }

    /// `w` and `b`: words are whitespace-separated runs, a CJK character is a word
    /// of its own, and both cross lines — the same rule the wrapper uses, so the two
    /// never disagree about where a word begins.
    #[test]
    fn words_are_walked_forwards_and_backwards() {
        let f = font();
        let blocks = Blocks::new(&f);
        let mut t = Transcript::default();
        t.ask("q", &f);
        // Two rows: the second is a different line of the same block.
        t.delta("alpha beta\ngamma delta", &f, 400.0);
        t.finish_answer(&f);
        let rows: Vec<String> = t.rows(blocks).iter().map(|r| r.text.to_string()).collect();
        assert_eq!(rows, ["alpha beta", "gamma delta"], "two lines as written");

        // Forwards: the start of each word, and across the line boundary.
        let mut at = (0, 0);
        for want in [(0, 6), (1, 0), (1, 6)] {
            at = t.word_forward(at, blocks);
            assert_eq!(at, want, "word start");
        }
        assert_eq!(
            t.word_forward(at, blocks),
            (1, 11),
            "the end when there is more"
        );

        // Backwards: the same starts, in reverse, and from the middle of a word.
        let mut at = (1, 11);
        for want in [(1, 6), (1, 0), (0, 6), (0, 0)] {
            at = t.word_back(at, blocks);
            assert_eq!(at, want, "word start backwards");
        }
        assert_eq!(t.word_back(at, blocks), (0, 0), "the first word");

        // CJK: one character is one word.
        let mut c = Transcript::default();
        c.ask("q", &f);
        c.delta("\u{4e2d}\u{6587}\u{6d4b}\u{8bd5}", &f, 400.0);
        c.finish_answer(&f);
        assert_eq!(
            c.word_forward((0, 0), blocks),
            (0, 3),
            "the second ideograph"
        );
        assert_eq!(c.word_forward((0, 3), blocks), (0, 6), "the third");
        assert_eq!(c.word_back((0, 6), blocks), (0, 3));
    }

    /// A row knows where it is, because the viewport scrolls in pixels.
    #[test]
    fn a_row_reports_the_pixel_it_starts_at() {
        let f = font();
        let blocks = Blocks::new(&f);
        let mut t = Transcript::default();
        t.ask("why", &f);
        t.delta("because", &f, 400.0);
        t.finish_answer(&f);
        t.fail("and then it broke", &f, 400.0);
        for index in 0..t.total_rows() {
            let y = t.row_y(index, blocks).expect("every row has a place");
            let row = t.rows(blocks)[index];
            assert_eq!(row.y, y, "the row and its place agree");
        }
        assert_eq!(t.row_y(t.total_rows(), blocks), None);
    }

    /// The line `Alt+C` names inside a card is the answer's own text, never the
    /// padding the card puts above it.
    #[test]
    fn a_line_inside_a_card_is_the_line_the_highlight_names() {
        let f = font();
        let blocks = Blocks::new(&f);
        let mut t = Transcript::default();
        t.ask("why", &f);
        t.delta("because", &f, 400.0);
        t.finish_answer(&f);
        let rows = t.rows(blocks);
        let first_answer = rows
            .iter()
            .position(|r| r.kind == Kind::Answer)
            .expect("an answer row");
        assert_eq!(rows[first_answer].text, "because");
        assert_eq!(
            rows[first_answer].y, rows[first_answer].block.0,
            "the answer's first row starts at the block"
        );
        assert_eq!(t.row_y(first_answer, blocks), Some(rows[first_answer].y));
    }

    #[test]
    fn a_reply_larger_than_the_cap_is_cut_and_marked() {
        let f = font();
        let mut t = Transcript::default();
        t.ask("q", &f);
        // 100 KiB in 1 KiB deltas: more than the cap, and streamed in pieces.
        let chunk = "x".repeat(1024);
        for _ in 0..100 {
            t.delta(&chunk, &f, 400.0);
        }
        t.finish_answer(&f);
        let kept: usize = t.rows(Blocks::new(&f)).iter().map(|r| r.text.len()).sum();
        assert!(kept <= REPLY_CAP + 8, "the cap held: {kept} bytes");
        assert!(
            text_of(&t, &f).iter().any(|r| r.contains('\u{2026}')),
            "the cut is visible"
        );
        assert!(t.entries()[1].truncated);
    }

    #[test]
    fn the_cap_does_not_touch_earlier_turns() {
        let f = font();
        let mut t = Transcript::default();
        t.ask("first question", &f);
        t.delta("a short answer", &f, 400.0);
        t.finish_answer(&f);
        t.ask("second question", &f);
        for _ in 0..100 {
            t.delta(&"y".repeat(1024), &f, 400.0);
        }
        t.finish_answer(&f);
        assert_eq!(t.request_turns()[0].text, "first question");
        assert_eq!(t.request_turns()[1].text, "a short answer");
    }

    #[test]
    fn a_resize_lays_the_same_text_out_at_the_new_width() {
        let f = font();
        let mut t = Transcript::default();
        t.ask(
            "a question that is long enough to wrap at a narrow width",
            &f,
        );
        t.delta(
            "and an answer that is also long enough to wrap when the panel narrows",
            &f,
            600.0,
        );
        t.finish_answer(&f);
        let wide = text_of(&t, &f);
        t.rewrap(&f, 120.0);
        let narrow = text_of(&t, &f);
        assert!(narrow.len() > wide.len(), "narrower must mean more rows");
        for row in &narrow {
            assert!(render_width(&f, row) <= 120.0, "row overflows: {row:?}");
        }
        // The text itself is unchanged: the same request goes back out.
        t.rewrap(&f, 600.0);
        assert_eq!(text_of(&t, &f), wide);
    }

    fn render_width(f: &PanelFont, s: &str) -> f32 {
        crate::render::measure(f, s, f32::MAX)
    }

    #[test]
    fn a_multibyte_boundary_is_never_split() {
        let (head, cut) = split_at_char_boundary("中文", 3);
        assert_eq!(head, "中");
        assert!(cut, "a byte was dropped");
        assert_eq!(split_at_char_boundary("abc", 9), ("abc", false));
        assert_eq!(split_at_char_boundary("abc", 0), ("", true));
    }

    #[test]
    fn a_new_question_closes_the_answer_left_open() {
        let f = font();
        let mut t = Transcript::default();
        t.ask("one", &f);
        t.delta("streaming", &f, 400.0);
        t.ask("two", &f);
        assert!(
            !t.entries()[1].complete,
            "the abandoned answer is not complete"
        );
        assert!(
            t.request_turns().iter().all(|x| x.text != "streaming"),
            "and it is not sent back"
        );
        assert!(t.is_streaming(), "the new answer is the open one");
    }
}
