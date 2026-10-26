//! Provider configuration, the request body, and the curl configuration that
//! carries both to the sandboxed child.
//!
//! The credential never reaches a command line: it is written into the curl
//! configuration, which the child reads from stdin.

use std::path::PathBuf;

pub const DEFAULT_BASE_URL: &str = "https://api.commandcode.ai/provider/v1";
pub const DEFAULT_MODEL: &str = "deepseek/deepseek-v4.1-flash";
/// The only environment variable the panel reads for authentication.
pub const KEY_VAR: &str = "COMMAND_CODE_API_KEY";
pub const MODEL_VAR: &str = "ASIDE_MODEL";
pub const BASE_URL_VAR: &str = "ASIDE_BASE_URL";
pub const PROXY_VAR: &str = "ASIDE_PROXY";

/// Timeouts, as curl flags. curl cannot tell "no first byte" from "stopped
/// mid-stream" — both exit 28 — so the panel distinguishes them by whether any
/// byte arrived (`sse::Outcome`).
pub const CONNECT_TIMEOUT_S: u32 = 10;
pub const SPEED_LIMIT: u32 = 1;
pub const SPEED_TIME_S: u32 = 30;
pub const MAX_TIME_S: u32 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub base_url: String,
    pub model: String,
    /// Where the request goes instead of direct: any scheme `curl` accepts
    /// (`http://`, `socks5h://`, …). `None` is a direct connection, and inside
    /// the sandbox it is the only way to use a proxy at all — `--clearenv`
    /// keeps `http_proxy` and friends out.
    pub proxy: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            proxy: None,
        }
    }
}

/// A configuration file problem, with the line that caused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub path: PathBuf,
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.path.display(), self.line, self.message)
    }
}

/// Where the config file lives: `$XDG_CONFIG_HOME/aside/config.toml`, or
/// `~/.config/aside/config.toml`. Never written to.
pub fn config_path() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    Some(base.join("aside").join("config.toml"))
}

/// Pick one value from the four sources, highest first. A source that is present
/// and non-empty replaces every lower one whole.
pub fn pick(
    flag: Option<&str>,
    env: Option<String>,
    file: Option<&[(String, String)]>,
    key: &str,
) -> Option<String> {
    flag.map(str::to_string)
        .or_else(|| env.filter(|v| !v.trim().is_empty()))
        .or_else(|| file?.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()))
}

/// The configuration file's entries, for a caller that needs more than the
/// provider's own keys. Absent file is an empty list, not an error.
pub fn entries() -> Result<Vec<(String, String)>, ConfigError> {
    match config_path() {
        Some(path) => read_config(&path),
        None => Ok(Vec::new()),
    }
}

/// Resolve the model, base URL and proxy: flag, then environment, then config
/// file, then the built-in default.
pub fn resolve(
    flag_model: Option<&str>,
    flag_base_url: Option<&str>,
    flag_proxy: Option<&str>,
    file: &[(String, String)],
) -> Result<Config, ConfigError> {
    let file = file.to_vec();
    let defaults = Config::default();
    Ok(Config {
        model: pick(
            flag_model,
            std::env::var(MODEL_VAR).ok(),
            Some(&file),
            "model",
        )
        .unwrap_or(defaults.model),
        base_url: pick(
            flag_base_url,
            std::env::var(BASE_URL_VAR).ok(),
            Some(&file),
            "base_url",
        )
        .unwrap_or(defaults.base_url),
        proxy: pick(
            flag_proxy,
            std::env::var(PROXY_VAR).ok(),
            Some(&file),
            "proxy",
        ),
    })
}

/// The key, from the environment only. A config file can never hold one.
pub fn api_key() -> Option<String> {
    std::env::var(KEY_VAR).ok().filter(|k| !k.trim().is_empty())
}

fn read_config(path: &std::path::Path) -> Result<Vec<(String, String)>, ConfigError> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        // Absence is not an error: the environment may carry everything.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(ConfigError {
                path: path.to_path_buf(),
                line: 1,
                message: format!("cannot read: {e}"),
            });
        }
    };
    parse_config(&text).map_err(|(line, message)| ConfigError {
        path: path.to_path_buf(),
        line,
        message,
    })
}

/// Flat `key = "value"` lines with `#` comments. `model`, `base_url`, `proxy`,
/// `font_size`, `font_family` and `animations` are the only known keys; anything else
/// is an error
/// rather than a silent shrug, so a typo cannot quietly change nothing. A size is
/// validated here, where its line number is known.
pub fn parse_config(text: &str) -> Result<Vec<(String, String)>, (usize, String)> {
    let mut out = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        let s = raw.split('#').next().unwrap_or("").trim();
        if s.is_empty() {
            continue;
        }
        let Some((key, value)) = s.split_once('=') else {
            return Err((line, format!("expected `key = \"value\"`: {raw:?}")));
        };
        let (key, value) = (key.trim(), value.trim());
        let value = match value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
            Some(v) => v.to_string(),
            None => {
                return Err((line, format!("value of `{key}` must be quoted: {value:?}")));
            }
        };
        match key {
            "font_size" => {
                let size: f32 = value.parse().map_err(|_| {
                    (
                        line,
                        format!("`font_size` must be a number of pixels: {value:?}"),
                    )
                })?;
                let range = crate::font::SIZE_RANGE;
                if !range.contains(&(size as u32)) || size <= 0.0 {
                    return Err((
                        line,
                        format!(
                            "`font_size` must be between {} and {} pixels: {value:?}",
                            range.start(),
                            range.end()
                        ),
                    ));
                }
                out.push((key.to_string(), value));
            }
            "animations" => {
                if value != "true" && value != "false" {
                    return Err((
                        line,
                        format!("`animations` must be \"true\" or \"false\": {value:?}"),
                    ));
                }
                out.push((key.to_string(), value));
            }
            "model" | "base_url" | "proxy" | "font_family" => {
                out.push((key.to_string(), value));
            }
            other => return Err((line, format!("unknown key `{other}`"))),
        }
    }
    Ok(out)
}

/// One conversation turn, oldest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    pub role: Role,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

impl Role {
    fn wire(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

/// The OpenAI-compatible request body: this run's turns, in order, streamed.
pub fn request_body(turns: &[Turn], model: &str) -> Result<String, String> {
    let mut body = String::with_capacity(256);
    body.push_str("{\"model\":\"");
    body.push_str(&escape_json(model)?);
    body.push_str("\",\"messages\":[");
    for (i, turn) in turns.iter().enumerate() {
        if i > 0 {
            body.push(',');
        }
        body.push_str("{\"role\":\"");
        body.push_str(turn.role.wire());
        body.push_str("\",\"content\":\"");
        body.push_str(&escape_json(&turn.text)?);
        body.push_str("\"}");
    }
    body.push_str("],\"stream\":true}");
    Ok(body)
}

/// Escape for a JSON string, refusing control characters rather than inventing
/// escapes for them: a question carrying one is not something the panel should
/// hand to a provider.
pub fn escape_json(s: &str) -> Result<String, String> {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                return Err(format!("control character U+{:04X} refused", c as u32));
            }
            c => out.push(c),
        }
    }
    Ok(out)
}

/// Escape for a curl configuration value. This runs *after* JSON escaping, so it
/// escapes only what curl's own parser reads: a backslash and a double quote.
pub fn escape_curl_config(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The curl configuration the child reads on stdin: where to go, how to
/// authenticate, how long to wait, and the body. `-K -` keeps every one of these
/// out of the process table.
pub fn curl_config(config: &Config, key: &str, body: &str) -> String {
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
    let proxy = match &config.proxy {
        Some(p) => format!("proxy = \"{}\"\n", escape_curl_config(p)),
        None => String::new(),
    };
    format!(
        "url = \"{}\"\n\
         {proxy}\
         request = \"POST\"\n\
         header = \"Authorization: Bearer {}\"\n\
         header = \"content-type: application/json\"\n\
         no-buffer\n\
         connect-timeout = {CONNECT_TIMEOUT_S}\n\
         speed-limit = {SPEED_LIMIT}\n\
         speed-time = {SPEED_TIME_S}\n\
         max-time = {MAX_TIME_S}\n\
         silent\n\
         show-error\n\
         fail-with-body\n\
         data-binary = \"{}\"\n",
        escape_curl_config(&url),
        escape_curl_config(key),
        escape_curl_config(body),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_source_beats_the_one_below_it() {
        let file = vec![("model".to_string(), "file-model".to_string())];
        let env = Some("env-model".to_string());
        assert_eq!(
            pick(Some("flag-model"), env.clone(), Some(&file), "model").as_deref(),
            Some("flag-model")
        );
        assert_eq!(
            pick(None, env.clone(), Some(&file), "model").as_deref(),
            Some("env-model")
        );
        assert_eq!(
            pick(None, None, Some(&file), "model").as_deref(),
            Some("file-model")
        );
        assert_eq!(pick(None, None, Some(&file), "base_url"), None);
        // An empty environment variable is not a choice.
        assert_eq!(
            pick(None, Some("  ".to_string()), Some(&file), "model").as_deref(),
            Some("file-model")
        );
    }

    #[test]
    fn the_defaults_are_the_documented_ones() {
        let c = Config::default();
        assert_eq!(c.model, "deepseek/deepseek-v4.1-flash");
        assert_eq!(c.base_url, "https://api.commandcode.ai/provider/v1");
        assert_eq!(c.proxy, None, "a bare machine goes direct");
        assert_eq!(KEY_VAR, "COMMAND_CODE_API_KEY");
        assert_eq!(PROXY_VAR, "ASIDE_PROXY");
    }

    #[test]
    fn config_parses_flat_keys_comments_and_blank_lines() {
        let parsed = parse_config(
            "# what to ask\nmodel = \"deepseek/deepseek-v4-flash\"\n\nbase_url = \"https://x/v1\" # trailing\nproxy = \"socks5h://127.0.0.1:1080\"\n",
        )
        .expect("parses");
        assert_eq!(
            parsed,
            vec![
                (
                    "model".to_string(),
                    "deepseek/deepseek-v4-flash".to_string()
                ),
                ("base_url".to_string(), "https://x/v1".to_string()),
                ("proxy".to_string(), "socks5h://127.0.0.1:1080".to_string()),
            ]
        );
    }

    /// The font keys live in the same file, and a size that is not a size is a
    /// configuration error with its line — the panel must not open at a size nobody
    /// asked for.
    #[test]
    fn the_font_keys_are_read_and_a_bad_size_is_refused() {
        let ok = parse_config("font_size = \"28\"\nfont_family = \"DejaVu Sans Mono\"\n")
            .expect("valid");
        assert_eq!(
            ok,
            [
                ("font_size".to_string(), "28".to_string()),
                ("font_family".to_string(), "DejaVu Sans Mono".to_string())
            ]
        );
        // The range is inclusive at both ends, and fractions are sizes too.
        for good in ["8", "72", "13.5"] {
            assert!(
                parse_config(&format!("font_size = \"{good}\"")).is_ok(),
                "{good} is a size"
            );
        }
        for bad in ["huge", "2", "73", "-4", ""] {
            let err = parse_config(&format!("font_size = \"{bad}\"")).expect_err(bad);
            assert_eq!(err.0, 1, "the line is named");
            assert!(err.1.contains("font_size"), "the key is named: {}", err.1);
        }
    }

    #[test]
    fn the_animation_key_takes_two_values_and_nothing_else() {
        for good in ["true", "false"] {
            assert!(
                parse_config(&format!("animations = \"{good}\"")).is_ok(),
                "{good} is a setting"
            );
        }
        let (line, message) = parse_config("animations = \"sometimes\"").expect_err("must fail");
        assert_eq!(line, 1, "the line is named");
        assert!(message.contains("animations"), "{message}");
        assert!(
            message.contains("false"),
            "both values are named: {message}"
        );
    }

    #[test]
    fn a_malformed_line_names_its_line_number() {
        let text = "model = \"a\"\nbase_url = https://x\n";
        let (line, message) = parse_config(text).expect_err("must fail");
        assert_eq!(line, 2, "the second line is the bad one: {message}");
    }

    #[test]
    fn an_unknown_key_is_refused_not_ignored() {
        let (line, message) = parse_config("api_key = \"secret\"\n").expect_err("must fail");
        assert_eq!(line, 1);
        assert!(message.contains("unknown key"), "{message}");
    }

    #[test]
    fn json_escaping_covers_quotes_backslashes_newlines_and_cjk() {
        assert_eq!(escape_json("a\"b").unwrap(), "a\\\"b");
        assert_eq!(escape_json("a\\b").unwrap(), "a\\\\b");
        assert_eq!(escape_json("a\nb").unwrap(), "a\\nb");
        assert_eq!(escape_json("折行").unwrap(), "折行");
        assert!(
            escape_json("bell\u{7}").is_err(),
            "control characters refused"
        );
    }

    #[test]
    fn curl_config_escaping_only_escapes_curl_metacharacters() {
        // One backslash from the JSON layer becomes two for curl, which reads
        // one back through: the body arrives unchanged.
        assert_eq!(escape_curl_config("a\\nb"), "a\\\\nb");
        assert_eq!(escape_curl_config("say \"hi\""), "say \\\"hi\\\"");
    }

    #[test]
    fn the_proxy_line_appears_only_when_one_is_configured() {
        let direct = curl_config(&Config::default(), "k", "{}");
        assert!(
            !direct.contains("proxy"),
            "a direct request carries no proxy line: {direct}"
        );
        let proxied = Config {
            proxy: Some("socks5h://127.0.0.1:1080".to_string()),
            ..Config::default()
        };
        let text = curl_config(&proxied, "k", "{}");
        // Its own line, between the URL and the request: a formatting slip here
        // would silently hand curl a broken configuration.
        assert!(
            text.contains(
                "url = \"https://api.commandcode.ai/provider/v1/chat/completions\"\n\
                 proxy = \"socks5h://127.0.0.1:1080\"\n\
                 request = \"POST\"\n"
            ),
            "{text}"
        );
        // The value goes through the same escaping as the URL and the key.
        let odd = Config {
            proxy: Some("http://p/\"x\"".to_string()),
            ..Config::default()
        };
        assert!(curl_config(&odd, "k", "{}").contains("proxy = \"http://p/\\\"x\\\"\"\n"));
    }

    #[test]
    fn the_body_carries_the_turns_in_order_and_streams() {
        let turns = vec![
            Turn {
                role: Role::User,
                text: "why".to_string(),
            },
            Turn {
                role: Role::Assistant,
                text: "because".to_string(),
            },
            Turn {
                role: Role::User,
                text: "and 折行?".to_string(),
            },
        ];
        let body = request_body(&turns, DEFAULT_MODEL).expect("escapes");
        assert_eq!(
            body,
            "{\"model\":\"deepseek/deepseek-v4.1-flash\",\"messages\":[{\"role\":\"user\",\"content\":\"why\"},{\"role\":\"assistant\",\"content\":\"because\"},{\"role\":\"user\",\"content\":\"and 折行?\"}],\"stream\":true}"
        );
    }

    #[test]
    fn the_curl_config_holds_the_credential_and_the_timeouts() {
        let config = Config::default();
        let text = curl_config(&config, "user_secret", "{\"a\":1}");
        assert!(text.contains("Authorization: Bearer user_secret"));
        assert!(text.contains("url = \"https://api.commandcode.ai/provider/v1/chat/completions\""));
        assert!(text.contains("data-binary = \"{\\\"a\\\":1}\""));
        assert!(text.contains(&format!("connect-timeout = {CONNECT_TIMEOUT_S}")));
        assert!(text.contains(&format!("speed-time = {SPEED_TIME_S}")));
        assert!(text.contains(&format!("max-time = {MAX_TIME_S}")));
        assert!(
            text.contains("no-buffer"),
            "the stream must not be buffered"
        );
    }

    #[test]
    fn a_64_kib_question_round_trips_through_both_escaping_layers() {
        let question: String = "折行 \"why\" \\ x\n".repeat(4000);
        assert!(question.len() > 64 * 1024);
        let body = request_body(
            &[Turn {
                role: Role::User,
                text: question.clone(),
            }],
            DEFAULT_MODEL,
        )
        .expect("escapes");
        let text = curl_config(&Config::default(), "k", &body);
        // The curl layer escapes exactly the JSON layer's backslashes and quotes.
        let escaped = escape_curl_config(&body);
        assert!(text.contains(&format!("data-binary = \"{escaped}\"")));
        assert_eq!(
            escaped.matches("\\\\\"").count(),
            body.matches("\\\"").count()
        );
    }
}
