//! The provider's stream: bytes in, ordered text deltas out.
//!
//! Written against the shape the provider actually sends (captured 2026-10-17):
//! `data: {...}` frames separated by bare `\n` (no CRLF, contrary to the common
//! claim), a first frame carrying only `{"role":"assistant"}`, reasoning frames
//! carrying `reasoning_content`, and a terminal `data: [DONE]`. Everything not
//! `data:` is ignored, and a frame with no text in it produces no text.

/// What one `push` produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Text to append to the answer.
    Delta(String),
    /// The provider reported an error inside the stream.
    Error(String),
    /// The provider said the answer is over.
    Done,
}

/// Whether the stream ended with the provider's terminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Complete,
    Truncated,
}

/// The most bytes one line may hold. A frame larger than this cannot be one the
/// panel would draw — the reply itself is capped at 64 KiB — so the reader stops
/// holding it instead of letting a provider grow the buffer without bound.
pub const MAX_LINE_BYTES: usize = 256 * 1024;

#[derive(Default)]
pub struct Reader {
    /// Bytes of a line that has not ended yet: a delta can be split across two
    /// reads, and the last line may come without a newline.
    partial: Vec<u8>,
    /// The current line passed `MAX_LINE_BYTES`; drop it through its newline.
    overflowing: bool,
    done: bool,
    received_any: bool,
}

impl Reader {
    /// Decode everything readable in `chunk`. Frames are delivered in order.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<Event> {
        if !chunk.is_empty() {
            self.received_any = true;
        }
        let mut events = Vec::new();
        let mut rest = chunk;
        while let Some(nl) = rest.iter().position(|b| *b == b'\n') {
            let (head, tail) = rest.split_at(nl);
            self.append(head);
            rest = &tail[1..];
            self.end_line(&mut events);
        }
        self.append(rest);
        events
    }

    /// The stream ended. Anything still buffered was an unterminated last line.
    pub fn finish(&mut self) -> Outcome {
        let mut events = Vec::new();
        self.end_line(&mut events);
        if self.done {
            Outcome::Complete
        } else {
            Outcome::Truncated
        }
    }

    /// Whether the provider sent anything at all (used to tell "no response"
    /// from "the stream stopped", since curl exits 28 for both).
    pub fn received_any(&self) -> bool {
        self.received_any
    }

    #[cfg(test)]
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Add bytes to the line being assembled, stopping at the cap: a line too
    /// long to be a frame is discarded to its newline, never held.
    fn append(&mut self, bytes: &[u8]) {
        if self.overflowing || bytes.is_empty() {
            return;
        }
        if self.partial.len() + bytes.len() > MAX_LINE_BYTES {
            self.overflowing = true;
            self.partial = Vec::new();
        } else {
            self.partial.extend_from_slice(bytes);
        }
    }

    fn end_line(&mut self, out: &mut Vec<Event>) {
        let mut line = std::mem::take(&mut self.partial);
        if std::mem::take(&mut self.overflowing) {
            return; // too long to be a frame; the next line starts clean
        }
        // A CRLF stream is the same stream: strip the carriage return.
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        self.line(&line, out);
    }

    fn line(&mut self, line: &[u8], out: &mut Vec<Event>) {
        let Ok(text) = std::str::from_utf8(line) else {
            return; // not a frame we can read; the next one may be
        };
        let Some(payload) = text.strip_prefix("data:") else {
            // Comments (`: OPENROUTER PROCESSING`), `event:` lines and blank
            // keep-alives carry no text.
            return;
        };
        let payload = payload.trim();
        if payload == "[DONE]" {
            self.done = true;
            out.push(Event::Done);
            return;
        }
        // A provider that reports a failure after a 200 sends its error object in
        // a `data:` frame; without this it would read as a frame with no text.
        if let Some(error) = error_message(payload) {
            out.push(Event::Error(error));
            return;
        }
        if let Some(content) = content_field(payload)
            && !content.is_empty()
        {
            out.push(Event::Delta(content));
        }
    }
}

/// The provider's in-stream error, when `payload` is its error object.
///
/// Only a payload that *is* the error object counts: a content string may mention
/// `error` without the stream failing.
fn error_message(payload: &str) -> Option<String> {
    if !payload.trim_start().starts_with("{\"error\"") {
        return None;
    }
    Some(
        string_field(payload, "message")
            .or_else(|| string_field(payload, "error"))
            .unwrap_or_else(|| "the provider reported an error".to_string()),
    )
}

/// The `content` of the first choice's delta, if the frame carries one.
///
/// This reads one field rather than parsing a document: the payload is a
/// single-shape OpenAI chunk, and the alternative is a JSON dependency for one
/// string. The key must match exactly, so `reasoning_content` — which this
/// provider streams before the answer — is not mistaken for it.
pub fn content_field(payload: &str) -> Option<String> {
    string_field(payload, "content")
}

/// The value of the first `"key": "..."` in `payload`, whatever the nesting:
/// the payloads here are single-shape chunks, and this reads one field rather
/// than building a document tree for one string.
pub fn string_field(payload: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let key = key.as_str();
    let mut from = 0;
    while let Some(at) = payload[from..].find(key) {
        let after_key = from + at + key.len();
        from = after_key;
        let rest = payload[after_key..].trim_start();
        let Some(rest) = rest.strip_prefix(':') else {
            continue;
        };
        let rest = rest.trim_start();
        if rest.starts_with("null") {
            return None; // final chunks carry `"content":null`
        }
        let Some(rest) = rest.strip_prefix('"') else {
            continue;
        };
        if let Some((value, _)) = json_string(rest) {
            return Some(value);
        }
    }
    None
}

/// Read a JSON string body, `rest` being the text after the opening quote.
fn json_string(rest: &str) -> Option<(String, &str)> {
    let mut out = String::with_capacity(rest.len().min(64));
    let mut chars = rest.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Some((out, &rest[i + 1..])),
            '\\' => {
                let (_, esc) = chars.next()?;
                match esc {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'u' => {
                        let hex: String = chars.by_ref().take(4).map(|(_, c)| c).collect();
                        let code = u32::from_str_radix(&hex, 16).ok()?;
                        // A surrogate pair is two escapes that make one char.
                        if (0xD800..0xDC00).contains(&code) {
                            // The low surrogate is its own escape: `\uYYYY`.
                            let it = chars.by_ref();
                            let (slash, u) = (it.next()?, it.next()?);
                            if slash.1 != '\\' || u.1 != 'u' {
                                return None;
                            }
                            let low_hex: String = it.take(4).map(|(_, c)| c).collect();
                            let low = u32::from_str_radix(&low_hex, 16).ok()?;
                            let combined =
                                0x10000 + ((code - 0xD800) << 10) + (low.wrapping_sub(0xDC00));
                            out.push(char::from_u32(combined)?);
                        } else {
                            out.push(char::from_u32(code)?);
                        }
                    }
                    other => out.push(other),
                }
            }
            c => out.push(c),
        }
    }
    None // unterminated: the frame was cut off, so it is not a value
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frames captured from the live provider on 2026-10-17, unedited.
    const ROLE_ONLY: &str = r#"data: {"id":"0fe7","object":"chat.completion.chunk","created":1792248231,"model":"deepseek/deepseek-v4.1-flash","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}],"system_fingerprint":"","sla_metrics":{"ttft_ms":2282,"ts_us":1792248231853099}}"#;
    const REASONING: &str = r#"data: {"id":"0fe7","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"role":"assistant","reasoning_content":" greeting"},"finish_reason":null}],"sla_metrics":{"ttft_ms":2282}}"#;
    const CONTENT: &str = r#"data: {"id":"0fe7","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
    const TRAILING: &str = r#"data: {"id":"0fe7","object":"chat.completion.chunk","choices":[],"system_fingerprint":"","usage":{"prompt_tokens":31,"completion_tokens":9}}"#;

    fn feed(reader: &mut Reader, chunks: &[&[u8]]) -> Vec<Event> {
        let mut out = Vec::new();
        for c in chunks {
            out.extend(reader.push(c));
        }
        out
    }

    #[test]
    fn a_role_only_frame_produces_no_text() {
        let mut r = Reader::default();
        let events = feed(&mut r, &[(ROLE_ONLY.to_string() + "\n").as_bytes()]);
        assert!(events.is_empty(), "{events:?}");
    }

    #[test]
    fn reasoning_content_is_not_the_answer() {
        let mut r = Reader::default();
        let events = feed(&mut r, &[(REASONING.to_string() + "\n").as_bytes()]);
        assert!(events.is_empty(), "reasoning must not become answer text");
    }

    #[test]
    fn content_becomes_a_delta() {
        let mut r = Reader::default();
        assert_eq!(
            feed(&mut r, &[(CONTENT.to_string() + "\n").as_bytes()]),
            vec![Event::Delta("Hello".to_string())]
        );
    }

    #[test]
    fn a_trailing_chunk_with_no_content_produces_no_text() {
        let mut r = Reader::default();
        assert!(feed(&mut r, &[(TRAILING.to_string() + "\n").as_bytes()]).is_empty());
        assert!(
            feed(
                &mut r,
                &[b"data: {\"choices\":[{\"delta\":{\"content\":null}}]}\n"]
            )
            .is_empty()
        );
    }

    #[test]
    fn the_done_sentinel_ends_the_answer() {
        let mut r = Reader::default();
        let events = feed(
            &mut r,
            &[(CONTENT.to_string() + "\n").as_bytes(), b"data: [DONE]\n\n"],
        );
        assert_eq!(
            events,
            vec![Event::Delta("Hello".to_string()), Event::Done],
            "the blank line after [DONE] is not a frame"
        );
        assert_eq!(r.finish(), Outcome::Complete);
        assert!(r.is_done());
    }

    #[test]
    fn a_stream_that_closes_without_done_is_truncated() {
        let mut r = Reader::default();
        feed(&mut r, &[(CONTENT.to_string() + "\n").as_bytes()]);
        assert_eq!(r.finish(), Outcome::Truncated);
    }

    #[test]
    fn a_delta_split_across_reads_is_reassembled() {
        // The real stream can split anywhere, including mid-payload.
        let frame = format!("{CONTENT}\ndata: [DONE]\n");
        let (a, b) = frame.split_at(60);
        let mut r = Reader::default();
        let mut events = feed(&mut r, &[a.as_bytes()]);
        assert!(events.is_empty(), "half a frame is not a frame yet");
        events.extend(r.push(b.as_bytes()));
        assert_eq!(events, vec![Event::Delta("Hello".to_string()), Event::Done]);
    }

    #[test]
    fn the_last_line_needs_no_newline() {
        let mut r = Reader::default();
        assert!(feed(&mut r, &[b"data: [DONE]"]).is_empty());
        assert_eq!(
            r.finish(),
            Outcome::Complete,
            "[DONE] counts without a newline"
        );
    }

    #[test]
    fn bare_lf_and_crlf_produce_the_same_answer() {
        let lf = format!("{CONTENT}\ndata: [DONE]\n");
        let crlf = lf.replace('\n', "\r\n");
        let mut a = Reader::default();
        let mut b = Reader::default();
        assert_eq!(
            feed(&mut a, &[lf.as_bytes()]),
            feed(&mut b, &[crlf.as_bytes()])
        );
        assert_eq!(a.finish(), b.finish());
    }

    #[test]
    fn comment_and_unknown_lines_are_ignored() {
        let mut r = Reader::default();
        let events = feed(
            &mut r,
            &[
                b": OPENROUTER PROCESSING\n",
                b"event: message\n",
                b"\n",
                b"not a frame\n",
            ],
        );
        assert!(events.is_empty(), "{events:?}");
    }

    #[test]
    fn escaping_in_a_content_string_is_decoded() {
        assert_eq!(
            content_field(r#"{"delta":{"content":"a\"b\\c\nd"}}"#).unwrap(),
            "a\"b\\c\nd"
        );
        assert_eq!(
            content_field(r#"{"delta":{"content":"\u4e2d\u6587"}}"#).unwrap(),
            "中文"
        );
        // A surrogate pair is one character.
        assert_eq!(
            content_field(r#"{"delta":{"content":"\ud83d\ude00"}}"#).unwrap(),
            "😀"
        );
        assert_eq!(content_field(r#"{"delta":{"content":""}}"#).unwrap(), "");
        assert_eq!(content_field(r#"{"delta":{"content":null}}"#), None);
    }

    #[test]
    fn a_frame_that_never_closes_its_string_is_not_a_value() {
        assert_eq!(content_field(r#"{"delta":{"content":"cut off"#), None);
    }

    #[test]
    fn no_content_frame_leaves_received_any_false() {
        let mut r = Reader::default();
        assert!(!r.received_any());
        r.push(b"");
        assert!(!r.received_any());
        r.push(b"\n");
        assert!(
            r.received_any(),
            "bytes arrived, even if they carry no text"
        );
    }

    #[test]
    fn a_line_of_exactly_the_cap_is_still_a_frame() {
        // The cap is on the line, not on what a frame may say: the boundary itself
        // must not be the point where parsing breaks.
        let mut line = String::from("data: [DONE]");
        line.push_str(&" ".repeat(MAX_LINE_BYTES - line.len()));
        assert_eq!(line.len(), MAX_LINE_BYTES);
        let mut r = Reader::default();
        let events = feed(&mut r, &[(line + "\n").as_bytes()]);
        assert_eq!(events, vec![Event::Done]);
    }

    #[test]
    fn a_line_beyond_the_cap_is_dropped_and_the_buffer_stays_bounded() {
        let mut r = Reader::default();
        // Fed in chunks, so the overflow is decided by accumulated bytes rather
        // than by one enormous read the OS happened to hand over.
        let chunk = vec![b'x'; 8 * 1024];
        let mut pushed = 0;
        while pushed <= MAX_LINE_BYTES {
            assert!(r.push(&chunk).is_empty());
            pushed += chunk.len();
            assert!(
                r.partial.len() <= MAX_LINE_BYTES,
                "the reader held {} bytes of one line",
                r.partial.len()
            );
        }
        assert!(r.overflowing, "the line passed the cap");
        assert!(
            r.push(b"\n").is_empty(),
            "a line too long to be a frame is not one"
        );
        assert!(
            !r.overflowing,
            "the newline ends it, so the next line starts clean"
        );
        assert_eq!(r.push(b"data: [DONE]\n"), vec![Event::Done]);
    }

    #[test]
    fn an_error_frame_is_a_failure_not_silence() {
        let mut r = Reader::default();
        let events = feed(
            &mut r,
            &[b"data: {\"error\":{\"code\":429,\"message\":\"rate limited\"}}\n"],
        );
        assert_eq!(events, vec![Event::Error("rate limited".to_string())]);
        assert_eq!(
            r.finish(),
            Outcome::Truncated,
            "no [DONE] arrived, so the stream is also unfinished"
        );
    }

    #[test]
    fn an_error_without_a_message_still_says_the_provider_failed() {
        let mut r = Reader::default();
        let events = feed(&mut r, &[b"data: {\"error\":{}}\n"]);
        assert_eq!(
            events,
            vec![Event::Error("the provider reported an error".to_string())]
        );
    }

    #[test]
    fn a_content_string_that_mentions_error_is_still_content() {
        // Only a payload that *is* the error object counts; the word inside the
        // model's own words must not turn the answer into a failure.
        let mut r = Reader::default();
        let events = feed(
            &mut r,
            &[b"data: {\"choices\":[{\"delta\":{\"content\":\"an error happened\"}}]}\n"],
        );
        assert_eq!(events, vec![Event::Delta("an error happened".to_string())]);
    }
}
