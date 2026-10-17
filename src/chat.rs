//! One provider request: the sandboxed child, the bytes it streams back, and
//! what to say when it fails.
//!
//! The caller drives this from the frame tick: `poll` drains what has arrived
//! and appends text to the transcript, `finish` reaps the child and reports how
//! the exchange ended.

use std::io::Read;
use std::process::Child;

use crate::feed::Feed;
use crate::font::PanelFont;
use crate::provider::{self, Config, Turn};
use crate::sandbox;
use crate::sse::{self, Reader};
use crate::transcript::Transcript;

/// How many bytes of a failed response are kept to explain the failure.
const ERROR_BODY_KEEP: usize = 8 * 1024;

/// How a request ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Close {
    /// The provider sent its terminator.
    Complete,
    /// The connection closed with the answer unfinished.
    Truncated(String),
    /// The request failed; the message is what the transcript shows.
    Failed(String),
}

pub struct Request {
    child: Child,
    feed: std::sync::Arc<Feed>,
    reader: Reader,
    /// The first bytes of the response, kept for the error path only.
    head: Vec<u8>,
}

impl Request {
    /// Start one request: a sandboxed curl, with the whole configuration —
    /// credential included — on its stdin.
    pub fn start(config: &Config, key: &str, turns: &[Turn]) -> Result<Request, String> {
        let body = provider::request_body(turns, &config.model)?;
        let mut child = sandbox::start().map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                "bwrap is not installed: refusing to run the request unsandboxed".to_string()
            }
            _ => format!("could not start the sandbox: {e}"),
        })?;
        let stdout = child.stdout.take().ok_or("no stdout from the sandbox")?;
        let feed = Feed::spawn(stdout);
        sandbox::write_config(&mut child, &provider::curl_config(config, key, &body))
            .map_err(|e| format!("could not hand the request to the sandbox: {e}"))?;
        Ok(Request {
            child,
            feed,
            reader: Reader::default(),
            head: Vec::new(),
        })
    }

    /// Take whatever has arrived and append it to the open answer. Returns
    /// whether anything changed on screen.
    pub fn poll(&mut self, transcript: &mut Transcript, font: &PanelFont, max_w: f32) -> bool {
        let bytes = self.feed.take();
        if bytes.is_empty() {
            return false;
        }
        if self.head.len() < ERROR_BODY_KEEP {
            let room = ERROR_BODY_KEEP - self.head.len();
            self.head.extend_from_slice(&bytes[..bytes.len().min(room)]);
        }
        let mut changed = false;
        for event in self.reader.push(&bytes) {
            match event {
                sse::Event::Delta(text) => {
                    transcript.delta(&text, font, max_w);
                    changed = true;
                }
                sse::Event::Done => {}
            }
        }
        changed
    }

    /// True while the child runs.
    pub fn running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Stop the request: the child dies, so there is nothing left delivering an
    /// answer the panel is no longer showing.
    pub fn cancel(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// Reap the child and say how the exchange ended. The completed text has
    /// already been handed to the transcript by `poll`.
    pub fn finish(&mut self) -> Close {
        let status = self.child.wait().ok().and_then(|s| s.code());
        let mut stderr = String::new();
        if let Some(mut e) = self.child.stderr.take() {
            let _ = e.read_to_string(&mut stderr);
        }
        // Anything still buffered in the reader is an unterminated last frame.
        let outcome = self.reader.finish();
        let received_any = self.reader.received_any();
        match status {
            Some(0) if outcome == sse::Outcome::Complete => Close::Complete,
            Some(0) => Close::Truncated(if received_any {
                "the connection closed before the answer finished".to_string()
            } else {
                "the provider sent no answer".to_string()
            }),
            Some(code) => Close::Failed(failure_message(
                code,
                &stderr,
                &String::from_utf8_lossy(&self.head),
                received_any,
            )),
            None => Close::Failed("the request ended without a status".to_string()),
        }
    }
}

/// Turn a curl exit status into something the person reading the panel can act
/// on, with the provider's own words where it gave any.
pub fn failure_message(exit: i32, stderr: &str, body: &str, received_any: bool) -> String {
    if sandbox::is_sandbox_failure(stderr) {
        return sandbox::sandbox_error(stderr);
    }
    match exit {
        // --fail-with-body: an HTTP error status. curl puts the status in
        // stderr and the provider's explanation in the body.
        22 => {
            let status = http_status(stderr);
            let detail = provider_message(body);
            let hint = match status {
                Some(401) | Some(403) => {
                    format!(" (check {})", provider::KEY_VAR)
                }
                _ => String::new(),
            };
            match (status, detail) {
                (Some(code), Some(msg)) => {
                    format!("the provider refused the request ({code}): {msg}{hint}")
                }
                (Some(code), None) => format!("the provider refused the request ({code}){hint}"),
                (None, Some(msg)) => format!("the provider refused the request: {msg}{hint}"),
                (None, None) => "the provider refused the request".to_string(),
            }
        }
        28 if received_any => "the answer stopped arriving part way through".to_string(),
        28 => format!(
            "the provider sent nothing within {}s",
            provider::CONNECT_TIMEOUT_S
        ),
        6 => "the provider's host name could not be resolved".to_string(),
        7 => "the provider could not be reached".to_string(),
        35 | 60 => "the provider's certificate could not be verified".to_string(),
        _ => {
            let line = stderr
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("the provider request failed");
            format!("the provider request failed: {}", strip_curl_prefix(line))
        }
    }
}

/// The HTTP status curl reported, out of `curl: (22) The requested URL returned
/// error: 401`.
fn http_status(stderr: &str) -> Option<u32> {
    let at = stderr.rfind("error: ")? + "error: ".len();
    let digits: String = stderr[at..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// The provider's own `message` field, if the error body carried one.
fn provider_message(body: &str) -> Option<String> {
    sse::string_field(body, "message").filter(|m| !m.trim().is_empty())
}

fn strip_curl_prefix(line: &str) -> String {
    match line.find(") ") {
        Some(at) if line.starts_with("curl:") => line[at + 2..].trim().to_string(),
        _ => line.trim().to_string(),
    }
}

/// Where the answer text is laid out. Kept here so the caller cannot forget that
/// the width changes with the window.
pub fn answer_width(panel_width: u32, scale: u32) -> f32 {
    let pad = crate::PAD * scale.max(1);
    panel_width.saturating_sub(pad * 2) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real exchange against the real provider. Off unless `ASIDE_LIVE=1`,
    /// because it costs a request: `ASIDE_LIVE=1 cargo test live` is the
    /// end-to-end check that runs without a compositor.
    #[test]
    fn a_live_exchange_streams_and_completes() {
        if std::env::var("ASIDE_LIVE").as_deref() != Ok("1") {
            return;
        }
        let key = provider::api_key().expect("COMMAND_CODE_API_KEY");
        let font = PanelFont::load(crate::font::SIZE).expect("font");
        let mut transcript = Transcript::default();
        transcript.ask("Reply with the single word: pong", &font, 600.0);
        let turns = transcript.request_turns();
        let mut request =
            Request::start(&Config::default(), &key, &turns).expect("the sandbox starts");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        while std::time::Instant::now() < deadline {
            request.poll(&mut transcript, &font, 600.0);
            if !request.running() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let close = request.finish();
        assert_eq!(close, Close::Complete, "the answer must end with [DONE]");
        let answer: String = transcript
            .rows()
            .iter()
            .map(|r| r.text)
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            answer.contains("pong"),
            "the model's words arrived: {answer}"
        );
    }

    /// The credential path that has to name the variable, against the real
    /// provider. Off unless `ASIDE_LIVE=1`.
    #[test]
    fn a_live_rejection_names_the_variable() {
        if std::env::var("ASIDE_LIVE").as_deref() != Ok("1") {
            return;
        }
        let font = PanelFont::load(crate::font::SIZE).expect("font");
        let turns = vec![provider::Turn {
            role: provider::Role::User,
            text: "hello".to_string(),
        }];
        let mut request =
            Request::start(&Config::default(), "cc_bogus_key_000", &turns).expect("starts");
        let mut transcript = Transcript::default();
        transcript.ask("hello", &font, 600.0);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while std::time::Instant::now() < deadline && request.running() {
            request.poll(&mut transcript, &font, 600.0);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let close = request.finish();
        match close {
            Close::Failed(message) => {
                assert!(message.contains("401"), "{message}");
                assert!(message.contains(provider::KEY_VAR), "{message}");
            }
            other => panic!("expected a named failure, got {other:?}"),
        }
    }

    /// The body the provider sent for a bad key, captured 2026-10-17.
    const UNAUTHORIZED: &str = r#"{"error":{"message":"Invalid 'Authorization' header or token.","type":"authentication_error","code":"UNAUTHORIZED"}}"#;

    #[test]
    fn an_http_error_names_the_status_and_the_providers_words() {
        let msg = failure_message(
            22,
            "curl: (22) The requested URL returned error: 401\n",
            UNAUTHORIZED,
            false,
        );
        assert!(msg.contains("401"), "{msg}");
        assert!(
            msg.contains("Invalid 'Authorization' header or token."),
            "{msg}"
        );
        assert!(
            msg.contains(provider::KEY_VAR),
            "the fix must be named: {msg}"
        );
    }

    #[test]
    fn a_rate_limit_is_reported_with_the_providers_words() {
        let body = r#"{"error":{"message":"Rate limit exceeded, try again later","type":"rate_limit_error"}}"#;
        let msg = failure_message(
            22,
            "curl: (22) The requested URL returned error: 429\n",
            body,
            false,
        );
        assert!(
            msg.contains("429") && msg.contains("Rate limit exceeded"),
            "{msg}"
        );
    }

    #[test]
    fn a_silent_timeout_differs_from_a_stalled_stream() {
        let no_bytes = failure_message(28, "curl: (28) Operation timed out\n", "", false);
        let stalled = failure_message(28, "curl: (28) Operation timed out\n", "", true);
        assert!(no_bytes.contains("sent nothing"), "{no_bytes}");
        assert!(stalled.contains("stopped arriving"), "{stalled}");
        assert_ne!(no_bytes, stalled);
    }

    #[test]
    fn name_resolution_and_connection_failures_are_distinct() {
        assert!(
            failure_message(6, "curl: (6) Could not resolve host: x\n", "", false)
                .contains("could not be resolved")
        );
        assert!(
            failure_message(7, "curl: (7) Failed to connect\n", "", false)
                .contains("could not be reached")
        );
        assert!(
            failure_message(35, "curl: (35) TLS connect error\n", "", false)
                .contains("certificate")
        );
    }

    #[test]
    fn an_unknown_exit_keeps_curls_own_line_without_the_prefix() {
        let msg = failure_message(
            18,
            "curl: (18) transfer closed with outstanding read data\n",
            "",
            false,
        );
        assert!(msg.contains("transfer closed"), "{msg}");
        assert!(!msg.contains("curl:"), "{msg}");
    }

    #[test]
    fn a_sandbox_start_failure_is_named_as_such() {
        let msg = failure_message(
            1,
            "bwrap: setting up uid map: Permission denied\n",
            "",
            false,
        );
        assert!(msg.starts_with("provider request not sent:"), "{msg}");
    }

    #[test]
    fn no_message_ever_carries_a_credential() {
        // curl never sees the key in an argument, and nothing here echoes the
        // configuration: this asserts the shape, and the argv test covers the rest.
        let key = "user_supersecret";
        for (exit, stderr, body) in [
            (
                22,
                "curl: (22) The requested URL returned error: 401\n",
                UNAUTHORIZED,
            ),
            (28, "curl: (28) time out\n", ""),
            (7, "curl: (7) Failed to connect\n", ""),
        ] {
            let msg = failure_message(exit, stderr, body, true);
            assert!(!msg.contains(key), "{msg}");
            assert!(!msg.contains("Bearer"), "{msg}");
        }
    }

    #[test]
    fn the_answer_width_follows_the_window_and_the_scale() {
        assert_eq!(answer_width(900, 1), 900.0 - 2.0 * crate::PAD as f32);
        assert_eq!(answer_width(1800, 2), 1800.0 - 4.0 * crate::PAD as f32);
        assert_eq!(answer_width(10, 1), 0.0, "a tiny window still lays out");
    }
}
