//! The conversation: turns in order, the open answer streaming into the last
//! entry, and what may be sent back to the provider.

use crate::font::PanelFont;
use crate::layout::Lines;
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
}

#[derive(Debug, Default)]
pub struct Transcript {
    entries: Vec<Entry>,
    /// Index of the answer being streamed into, if any.
    open: Option<usize>,
}

/// One row as the painter needs it: the text, and what it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row<'a> {
    pub text: &'a str,
    pub kind: Kind,
    /// First row of its entry: the painter spaces turns apart here.
    pub first: bool,
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

    fn push(&mut self, kind: Kind, complete: bool) -> usize {
        self.entries.push(Entry {
            kind,
            text: String::new(),
            lines: Lines::default(),
            complete,
            truncated: false,
        });
        self.entries.len() - 1
    }

    /// Lay every entry out again: a resize changes the width, and the wrap is a
    /// property of the width, not of the text.
    pub fn rewrap(&mut self, font: &PanelFont, max_w: f32) {
        for entry in &mut self.entries {
            entry.lines = Lines::wrap(&entry.text, font, max_w);
        }
    }

    /// Add a question and open an answer for it, so a delta has somewhere to go.
    pub fn ask(&mut self, question: &str, font: &PanelFont, max_w: f32) {
        let user = self.push(Kind::Question, true);
        self.entries[user].text.push_str(question);
        self.entries[user].lines.push(question, font, max_w);
        self.entries[user].lines.finish();
        // A previous answer that never finished is closed before a new question:
        // two open answers would have no defined meaning.
        self.close_open();
        self.open = Some(self.push(Kind::Answer, false));
    }

    /// Append streamed text to the open answer.
    pub fn delta(&mut self, text: &str, font: &PanelFont, max_w: f32) {
        let Some(i) = self.open else { return };
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
    }

    /// The answer ended normally.
    pub fn finish_answer(&mut self) {
        if let Some(i) = self.open.take() {
            self.entries[i].lines.finish();
            self.entries[i].complete = true;
        }
    }

    /// The answer was cancelled or the stream was cut: it stays on screen, and
    /// it is not sent back to the provider as if it were an answer.
    pub fn cancel(&mut self, note: &str, font: &PanelFont, max_w: f32) {
        if let Some(i) = self.open.take() {
            self.entries[i].lines.finish();
            self.add_note(note, font, max_w);
        }
    }

    /// A failure: its own entry, distinct from answer text.
    pub fn fail(&mut self, message: &str, font: &PanelFont, max_w: f32) {
        self.close_open();
        let i = self.push(Kind::Error, true);
        self.entries[i].text.push_str(message);
        self.entries[i].lines.push(message, font, max_w);
        self.entries[i].lines.finish();
    }

    /// Something the panel says about itself.
    pub fn add_note(&mut self, text: &str, font: &PanelFont, max_w: f32) {
        let i = self.push(Kind::Note, true);
        self.entries[i].text.push_str(text);
        self.entries[i].lines.push(text, font, max_w);
        self.entries[i].lines.finish();
    }

    fn close_open(&mut self) {
        if let Some(i) = self.open.take() {
            self.entries[i].lines.finish();
        }
    }

    /// Rows in display order, with the styling the painter needs.
    pub fn rows(&self) -> Vec<Row<'_>> {
        let mut out = Vec::new();
        for entry in &self.entries {
            for (i, row) in entry.lines.visible_rows().enumerate() {
                out.push(Row {
                    text: row,
                    kind: entry.kind,
                    first: i == 0,
                });
            }
        }
        out
    }

    pub fn total_rows(&self) -> usize {
        self.entries.iter().map(|e| e.lines.total_rows()).sum()
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

    fn text_of(t: &Transcript) -> Vec<String> {
        t.rows().iter().map(|r| r.text.to_string()).collect()
    }

    #[test]
    fn a_question_and_its_answer_stay_in_order() {
        let f = font();
        let mut t = Transcript::default();
        t.ask("why", &f, 400.0);
        t.delta("because", &f, 400.0);
        t.finish_answer();
        t.ask("and?", &f, 400.0);
        t.delta("this too", &f, 400.0);
        t.finish_answer();
        assert_eq!(text_of(&t), ["why", "because", "and?", "this too"]);
        assert!(!t.is_streaming());
    }

    #[test]
    fn the_request_carries_the_finished_turns_in_order() {
        let f = font();
        let mut t = Transcript::default();
        t.ask("first", &f, 400.0);
        t.delta("answer one", &f, 400.0);
        t.finish_answer();
        t.ask("second", &f, 400.0);
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
        t.ask("first", &f, 400.0);
        t.delta("half an ans", &f, 400.0);
        t.cancel("cancelled", &f, 400.0);
        t.ask("second", &f, 400.0);
        t.delta("full answer", &f, 400.0);
        t.finish_answer();
        let rows = text_of(&t);
        assert!(rows.contains(&"half an ans".to_string()), "still visible");
        assert!(rows.contains(&"cancelled".to_string()));
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
        t.ask("q", &f, 400.0);
        t.delta("partial", &f, 400.0);
        t.fail("connection lost", &f, 400.0);
        let turns = t.request_turns();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].text, "q");
        assert!(text_of(&t).iter().any(|r| r == "connection lost"));
    }

    #[test]
    fn a_failure_is_its_own_entry_kind() {
        let f = font();
        let mut t = Transcript::default();
        t.fail("401 unauthorized: COMMAND_CODE_API_KEY", &f, 400.0);
        let rows = t.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, Kind::Error);
        assert!(rows[0].first);
    }

    #[test]
    fn a_reply_larger_than_the_cap_is_cut_and_marked() {
        let f = font();
        let mut t = Transcript::default();
        t.ask("q", &f, 400.0);
        // 100 KiB in 1 KiB deltas: more than the cap, and streamed in pieces.
        let chunk = "x".repeat(1024);
        for _ in 0..100 {
            t.delta(&chunk, &f, 400.0);
        }
        t.finish_answer();
        let kept: usize = t.rows().iter().map(|r| r.text.len()).sum();
        assert!(kept <= REPLY_CAP + 8, "the cap held: {kept} bytes");
        assert!(
            text_of(&t).iter().any(|r| r.contains('\u{2026}')),
            "the cut is visible"
        );
        assert!(t.entries()[1].truncated);
    }

    #[test]
    fn the_cap_does_not_touch_earlier_turns() {
        let f = font();
        let mut t = Transcript::default();
        t.ask("first question", &f, 400.0);
        t.delta("a short answer", &f, 400.0);
        t.finish_answer();
        t.ask("second question", &f, 400.0);
        for _ in 0..100 {
            t.delta(&"y".repeat(1024), &f, 400.0);
        }
        t.finish_answer();
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
            600.0,
        );
        t.delta(
            "and an answer that is also long enough to wrap when the panel narrows",
            &f,
            600.0,
        );
        t.finish_answer();
        let wide = text_of(&t);
        t.rewrap(&f, 120.0);
        let narrow = text_of(&t);
        assert!(narrow.len() > wide.len(), "narrower must mean more rows");
        for row in &narrow {
            assert!(render_width(&f, row) <= 120.0, "row overflows: {row:?}");
        }
        // The text itself is unchanged: the same request goes back out.
        t.rewrap(&f, 600.0);
        assert_eq!(text_of(&t), wide);
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
        t.ask("one", &f, 400.0);
        t.delta("streaming", &f, 400.0);
        t.ask("two", &f, 400.0);
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
