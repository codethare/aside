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
    /// Whether the request went through a proxy, which is what decides which hop
    /// a connect-phase timeout belongs to.
    via_proxy: bool,
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
            via_proxy: config.proxy.is_some(),
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
                self.via_proxy,
            )),
            None => Close::Failed("the request ended without a status".to_string()),
        }
    }
}

/// Turn a curl exit status into something the person reading the panel can act
/// on, with the provider's own words where it gave any.
pub fn failure_message(
    exit: i32,
    stderr: &str,
    body: &str,
    received_any: bool,
    via_proxy: bool,
) -> String {
    if sandbox::is_sandbox_failure(stderr) {
        return sandbox::sandbox_error(stderr);
    }
    match exit {
        // 407 is the one status a proxy generates about itself, so through a proxy
        // it is that hop refusing, and "the provider refused" would name the wrong
        // one. Without a proxy there is no proxy hop to blame, so it falls through.
        22 if via_proxy && http_status(stderr) == Some(407) => match provider_message(body) {
            Some(msg) => format!("the proxy refused the request (407): {msg}"),
            None => "the proxy refused the request (407)".to_string(),
        },
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
        // "Connection timed out" is the connect phase, so TCP, the proxy CONNECT
        // and TLS were all still ahead of the request: it was never sent. curl
        // says the same words whichever hop was stuck (measured), so which one to
        // name follows from whether a proxy was in the way.
        28 if stderr.contains("Connection timed out") && via_proxy => format!(
            "the request never got through the proxy ({}s without a connection)",
            provider::CONNECT_TIMEOUT_S
        ),
        28 if stderr.contains("Connection timed out") => format!(
            "the provider could not be reached within {}s",
            provider::CONNECT_TIMEOUT_S
        ),
        28 => format!(
            // Not the connect deadline: this is the stall detector (speed-time),
            // which is what fires when the request went out and nothing came back.
            "the provider sent nothing within {}s",
            provider::SPEED_TIME_S
        ),
        6 => "the provider's host name could not be resolved".to_string(),
        // The proxy is the failing hop when one is configured: every proxy claim
        // this makes is gated on that, never on the wording (curl names the proxy
        // in most of these, but a provider host may read "proxy" too).
        5 | 97 if via_proxy => proxy_failure(stderr),
        7 if via_proxy => proxy_failure(stderr),
        7 => "the provider could not be reached".to_string(),
        35 | 60 => "the provider's certificate could not be verified".to_string(),
        _ => format!(
            "the provider request failed: {}",
            curl_words(stderr, "the provider request failed")
        ),
    }
}

/// The proxy hop failed: curl's own words, with any credential removed.
fn proxy_failure(stderr: &str) -> String {
    format!(
        "the request through the proxy failed: {}",
        curl_words(stderr, "the proxy request failed")
    )
}

/// curl's first line, without its `curl: (N)` prefix and without any credential
/// a URL in it carries: curl quotes addresses in errors, and a proxy or base URL
/// may well be `scheme://user:pass@host`.
fn curl_words(stderr: &str, fallback: &str) -> String {
    let line = stderr
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(fallback);
    scrub_credentials(&strip_curl_prefix(line))
}

/// Drop the `user:pass@` of every URL in `text`, keeping the host. An error that
/// quotes an address must not turn its credential into a disclosed one.
fn scrub_credentials(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(scheme_end) = rest.find("://") {
        let (scheme, after) = rest.split_at(scheme_end + 3);
        out.push_str(scheme);
        // The authority runs to the first character a URL cannot hold raw there;
        // a port's colon is part of it.
        let end = after
            .find(['/', '?', '#', ' ', '\'', '"', '\\'])
            .unwrap_or(after.len());
        let (authority, tail) = after.split_at(end);
        match authority.rfind('@') {
            Some(creds) => out.push_str(&authority[creds + 1..]),
            None => out.push_str(authority),
        }
        rest = tail;
    }
    out.push_str(rest);
    out
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
            false,
        );
        assert!(
            msg.contains("429") && msg.contains("Rate limit exceeded"),
            "{msg}"
        );
    }

    #[test]
    fn a_silent_timeout_differs_from_a_stalled_stream() {
        let no_bytes = failure_message(28, "curl: (28) Operation timed out\n", "", false, false);
        let stalled = failure_message(28, "curl: (28) Operation timed out\n", "", true, false);
        assert!(no_bytes.contains("sent nothing"), "{no_bytes}");
        assert!(stalled.contains("stopped arriving"), "{stalled}");
        assert_ne!(no_bytes, stalled);
    }

    #[test]
    fn name_resolution_and_connection_failures_are_distinct() {
        assert!(
            failure_message(6, "curl: (6) Could not resolve host: x\n", "", false, false)
                .contains("could not be resolved")
        );
        assert!(
            failure_message(7, "curl: (7) Failed to connect\n", "", false, false)
                .contains("could not be reached")
        );
        assert!(
            failure_message(35, "curl: (35) TLS connect error\n", "", false, false)
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
            false,
        );
        assert!(msg.starts_with("provider request not sent:"), "{msg}");
    }

    #[test]
    fn a_proxy_failure_names_the_proxy_and_keeps_curls_words() {
        for (exit, stderr) in [
            (5, "curl: (5) Could not resolve proxy: p.example\n"),
            (
                5,
                "curl: (5) Unsupported proxy syntax in 'http://p.example:99999': Port number was not a decimal number between 0 and 65535\n",
            ),
            (
                7,
                "curl: (7) Unsupported proxy scheme for 'ftp://p.example:3128'\n",
            ),
            (
                7,
                "curl: (7) Failed to connect to api.example:443 over proxy 127.0.0.1 after 4 ms: Could not connect to server\n",
            ),
            (97, "curl: (97) CONNECT tunnel failed, response 403\n"),
        ] {
            let msg = failure_message(exit, stderr, "", false, true);
            assert!(msg.contains("proxy"), "{msg}");
            assert!(!msg.contains("provider"), "{msg}");
        }
        // A malformed address must not be reported as an unresolvable name.
        let malformed = failure_message(
            5,
            "curl: (5) Unsupported proxy syntax in 'http://p.example:99999': bad port\n",
            "",
            false,
            true,
        );
        assert!(malformed.contains("syntax"), "{malformed}");
        // With no proxy configured, curl's proxy codes are not attributed to a hop
        // that is not there: they fall through to curl's own words.
        let no_proxy = failure_message(
            5,
            "curl: (5) Could not resolve proxy: p.example\n",
            "",
            false,
            false,
        );
        assert!(no_proxy.contains("request failed"), "{no_proxy}");
        assert!(!no_proxy.contains("through the proxy"), "{no_proxy}");
    }

    #[test]
    fn a_provider_exit_7_without_a_proxy_still_names_the_provider() {
        let msg = failure_message(
            7,
            "curl: (7) Failed to connect to api.example port 443 after 30000 ms: Timeout was reached\n",
            "",
            false,
            false,
        );
        assert!(msg.contains("provider"), "{msg}");
        assert!(!msg.contains("proxy"), "{msg}");
    }

    #[test]
    fn a_quoted_url_loses_its_credential_before_it_is_shown() {
        // curl quotes addresses in its errors, and both the proxy URL and the
        // base URL may carry `user:pass@`, which must never reach the transcript.
        let msg = failure_message(
            5,
            "curl: (5) Unsupported proxy syntax in 'http://alice:hunter2@proxy.example:99999': bad port\n",
            "",
            false,
            false,
        );
        assert!(!msg.contains("hunter2") && !msg.contains("alice"), "{msg}");
        assert!(msg.contains("proxy.example:99999"), "the host stays: {msg}");
        // The catch-all arm scrubs what it echoes.
        let other = failure_message(
            3,
            "curl: (3) bad range in position 13: http://alice:hunter2@[bad\n",
            "",
            false,
            false,
        );
        assert!(!other.contains("hunter2"), "{other}");
        assert!(other.contains("http://[bad"), "{other}");
    }

    #[test]
    fn scrubbing_keeps_everything_that_is_not_a_credential() {
        assert_eq!(
            scrub_credentials("http://alice:pw@h.example:8080/x"),
            "http://h.example:8080/x"
        );
        assert_eq!(
            scrub_credentials("socks5h://a:b@h.example"),
            "socks5h://h.example"
        );
        // Every URL in the line, not just the first.
        assert_eq!(
            scrub_credentials("see http://a:b@h.example and socks5://c:d@i.example:1"),
            "see http://h.example and socks5://i.example:1"
        );
        // An `@` outside an authority is not a credential, and an IPv6 host
        // keeps its brackets.
        assert_eq!(
            scrub_credentials("http://h.example/p@th"),
            "http://h.example/p@th"
        );
        assert_eq!(
            scrub_credentials("http://a:b@[::1]:8080"),
            "http://[::1]:8080"
        );
        assert_eq!(scrub_credentials("no url here"), "no url here");
    }

    #[test]
    fn a_connect_timeout_names_the_hop_that_was_stuck() {
        // Measured: curl prints the same words whether the proxy never answered
        // CONNECT, the tunnel came up and TLS never finished, or the provider's
        // address was simply unreachable — so the hop comes from the config.
        let timed_out = "curl: (28) Connection timed out after 10001 milliseconds\n";
        let direct = failure_message(28, timed_out, "", false, false);
        assert!(direct.contains("provider"), "{direct}");
        assert!(!direct.contains("proxy"), "{direct}");

        let proxied = failure_message(28, timed_out, "", false, true);
        assert!(proxied.contains("proxy"), "{proxied}");
        assert!(!proxied.contains("provider"), "{proxied}");
        assert_ne!(direct, proxied);
    }

    #[test]
    fn a_stalled_provider_is_the_providers_fault_with_or_without_a_proxy() {
        // The connect phase finished, so the connection was up — through a proxy or
        // direct — and the silence is the far end's. curl's stall wording is the
        // same either way, so the verdict is too: the provider, never the proxy.
        let stalled = "curl: (28) Operation too slow. Less than 1 bytes/sec transferred the last 30 seconds\n";
        let expected = format!(
            "the provider sent nothing within {}s",
            provider::SPEED_TIME_S
        );
        // Direct, the case named first.
        assert_eq!(failure_message(28, stalled, "", false, false), expected);
        // And the reverse: through a proxy, where the tunnel is up and it is the
        // provider behind it that went quiet.
        assert_eq!(failure_message(28, stalled, "", false, true), expected);
        assert!(!expected.contains("proxy"), "{expected}");
        // Once bytes have arrived it is a mid-answer stall, whoever carried it.
        let mid = failure_message(28, stalled, "", true, true);
        assert!(mid.contains("stopped arriving"), "{mid}");
        assert!(!mid.contains("proxy"), "{mid}");
    }

    #[test]
    fn the_hop_comes_from_the_config_not_from_curls_words() {
        // A provider whose host merely reads "proxy" must not be mistaken for the
        // proxy hop: with no proxy configured, exit 7 is the provider's failure.
        let named_proxy = "curl: (7) Failed to connect to proxy.example port 443 after 3000 ms: Couldn't connect to server\n";
        let direct = failure_message(7, named_proxy, "", false, false);
        assert!(direct.contains("provider"), "{direct}");
        assert!(!direct.contains("proxy"), "{direct}");
        // With one configured the same code is the proxy hop, and curl's words
        // name which address it could not reach.
        let over_proxy = "curl: (7) Failed to connect to api.example:443 over proxy 127.0.0.1 after 4 ms: Could not connect to server\n";
        let proxied = failure_message(7, over_proxy, "", false, true);
        assert!(proxied.contains("proxy"), "{proxied}");
        assert!(!proxied.contains("provider"), "{proxied}");
    }

    #[test]
    fn the_provider_side_paths_stay_the_providers_through_a_proxy() {
        // DNS, certificates and HTTP statuses arrive over an established tunnel,
        // so they are the provider's whichever hop carried them.
        for (exit, stderr) in [
            (6, "curl: (6) Could not resolve host: api.example\n"),
            (35, "curl: (35) TLS connect error\n"),
            (60, "curl: (60) SSL certificate problem\n"),
        ] {
            let msg = failure_message(exit, stderr, "", false, true);
            assert!(msg.contains("provider"), "{msg}");
            assert!(!msg.contains("proxy"), "{msg}");
        }
        let refused = failure_message(
            22,
            "curl: (22) The requested URL returned error: 503\n",
            "{\"message\":\"busy\"}",
            false,
            true,
        );
        assert!(refused.contains("provider"), "{refused}");
        assert!(!refused.contains("proxy"), "{refused}");
    }

    #[test]
    fn a_407_is_the_proxys_refusal_not_the_providers() {
        let stderr = "curl: (22) The requested URL returned error: 407\n";
        let body = "{\"message\":\"proxy auth required\"}";
        let proxied = failure_message(22, stderr, body, false, true);
        assert!(proxied.contains("proxy refused"), "{proxied}");
        assert!(!proxied.contains("provider"), "{proxied}");
        // No proxy configured: there is no proxy hop to blame, so the status is
        // reported as before.
        let direct = failure_message(22, stderr, body, false, false);
        assert!(direct.contains("provider refused"), "{direct}");
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
            let msg = failure_message(exit, stderr, body, true, false);
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
