# Tasks

## 1. Repository and crate skeleton

- [x] 1.1 Create `Cargo.toml` (name `aside`, `edition 2024`) with `smithay-client-toolkit`, `wayland-client`, `wayland-protocols` (with its `unstable` feature), `ab_glyph`, `fontdb`; add `rust-toolchain.toml` pinning 1.98.1 with rustfmt and clippy — verify: `cargo build` succeeds and `cargo tree --edges normal --prefix none | wc -l` is recorded in the README
- [x] 1.2 Write `CLAUDE.md` for this project: the conventions carried over from `wlmenu` (software rendering, jj, English, inline assertion tests, spec-then-plan, ask before adding a dependency), the four keys and the token table, and the explicit non-goals — verify: every rule in it is checkable, and the file names the commands it expects (`cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings`)
- [x] 1.3 Write `README.md` with what it does, how to run it, what it needs on `PATH` (`bwrap`, `curl`, `wl-copy`) and why it is built this way — verify: the documented commands are copy-pasteable and the dependency list matches `Cargo.toml`
- [x] 1.4 Add `src/main.rs` that prints usage and exits — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` all clean

## 2. Port the rendering core from wlmenu

- [x] 2.1 Port `font.rs` (family resolution, primary plus lazy fallback chain, glyph cache, subpixel coverage planes) and drop the `-f` spec syntax — verify: ported headless font tests pass, and a test asserts the 19 MB CJK face is not read when only Latin glyphs are requested
- [x] 2.2 Port the `render.rs` primitives (BGRA spans, rounded-outline clipping, glyph blit, blend helpers) and keep the WCAG contrast test and the compile-time content-inset assertion — verify: ported render tests pass; the const assertion fails to compile when the inset is lowered below the radius
- [x] 2.3 Port `anim.rs` unchanged and add the viewport model that bounds visible rows to 64 — verify: unit test asserts a viewport never holds more than 64 rows regardless of transcript length
- [x] 2.4 Port the `feed.rs` shape (reader thread, mutex queue, EOF flag drained by the frame tick) and use it for the `curl` stdout reader — verify: test drives the queue with a chunked producer and asserts ordering and an EOF flag

## 3. Layout: wrapping, rows, caret, highlight, multi-line input

- [x] 3.1 Implement width-measured line wrapping so Latin wraps at word boundaries and CJK wraps per character — verify: tests cover a long English paragraph, a CJK paragraph, and a mixed paragraph, each with no over-wide row. The rule implemented is the Latin + CJK subset of UAX #14 (breaks after spaces and between CJK characters), not the full table: the panel renders no shaped script, and the full table would need `unicode-linebreak`. The limit is marked in the code with a `shortcut:` comment
- [x] 3.2 Implement the tail-buffer incremental layout (`take_row`): appending a delta wraps only the growing tail — verify: test asserts wrapped rows and measured work stay linear as a 64 KiB answer arrives one delta at a time
- [x] 3.3 Implement the in-row caret, the single logical input line that soft-wraps, and the transcript line highlight — verify: tests assert caret position after insertion, deletion and word-jump, that `Up`/`Down` never move the caret, and that the highlight moves one line per `Up`/`Down`
- [x] 3.4 Implement the working indicator as a rail colour change (`LABEL_ACCENT` idle, `BG_SEL` while streaming) — verify: test renders a frame before and after a stream ends and asserts every transcript row is at the same pixel position

## 4. Wayland plumbing

- [x] 4.1 Registry, seat, keyboard, output plumbing and an `xdg-shell` toplevel that takes keyboard focus, with layer-shell specifics dropped — verify: no `wlr_layer` reference anywhere in the source (grep), and the toplevel's behaviour on a live session is item 1 of `docs/live-checklist.md`
- [x] 4.2 `wl_shm` pool with rotating slots, full-buffer repaint each frame, and `scale_factor_changed` re-deriving the font and re-wrapping — verify: the pool and full-buffer paint are in `main.rs`; the subpixel-only-at-1x rule is unit-tested in `layout`; the 1x/1.5x/2x behaviour on a live output is item 12 of the checklist
- [x] 4.3 Follow the `configure` size, and turn any `wl_display` error into an exit — verify: the window is mapped by a buffer-less first commit, every buffer is created for the configured size (so a size mismatch is not representable), and the event loop returns an error — printing it and exiting — on `Connection::protocol_error()` or a failed dispatch
- [ ] 4.4 Load fonts on a worker thread overlapped with the handshake — verify: a cold-start measurement with an empty transcript is under 50 ms to first frame, recorded in the run's notes
- [x] 4.5 Implement single-instance over a unix socket in `$XDG_RUNTIME_DIR` with inverted semantics (second launch raises via `xdg-activation` and exits zero; a stale socket is taken over) — verify: tests cover the claim, the stale-socket takeover and the raise message; a manual second launch focuses the existing window and creates no second one
- [x] 4.6 Open at 900×520 logical with a 420×200 minimum, follow the compositor's `configure` in both dimensions, and re-wrap the transcript exactly once per width change — verify: a test resizes a scripted transcript through three widths and asserts the wrapped rows match a fresh wrap at each, and that the frame after a resize is back to visible-row cost

## 5. IME client (`zwp_text_input_v3`)

- [x] 5.1 Implement the client with `wayland_protocols::wp::text_input::zv3`: enable/disable on keyboard enter and leave, `set_cursor_rectangle`, and handle `preedit_string`, `commit_string`, `delete_surrounding_text`, `done` with serial acknowledgement — verify: unit tests drive the state machine through a commit, a pre-edit and a surrounding-text deletion, and a 200-character CJK commit arrives intact in the input buffer
- [x] 5.2 Degrade when the compositor advertises no text-input manager: accept raw keys and report the missing IME — verify: test with no such global asserts the panel still constructs and the panel says IME input is unavailable
- [x] 5.3 Document the IME step for the user's session (fcitx or ibus running, a Chinese layout selected) in the manual checklist — verify: the checklist names the compositor, the IME and the exact question to type

## 6. Clipboard

- [x] 6.1 Spawn `wl-copy` with the highlighted line on stdin, and only on `Alt+C` — verify: a test with a stub `wl-copy` on `PATH` asserts exactly one child per keypress, that stdin holds exactly that line, and that no child is spawned on exit or on any other key
- [x] 6.2 Assert the panel never sets the clipboard or the primary selection itself — verify: a test asserts no clipboard call is reachable outside the `Alt+C` path

## 7. Provider request assembly

- [x] 7.1 Resolve provider, base URL, model and key with the precedence flag > env > config file > default, defaults `commandcode` / `https://api.commandcode.ai/provider/v1` / `deepseek/deepseek-v4.1-flash` — verify: tests cover each precedence pair and the bare-machine default
- [x] 7.2 Read `$XDG_CONFIG_HOME/aside/config.toml` (falling back to `~/.config/aside/config.toml`) as flat `key = "value"` lines with `#` comments, recognising `model` and `base_url`; never write it, tolerate absence, and fail with path plus line number on an unrecognised key or a malformed line — verify: tests for absent, valid, malformed-at-line-4 and unknown-key
- [x] 7.3 Read `COMMAND_CODE_API_KEY`, and only from there, refusing to open a connection when it is absent — verify: a run with the variable unset exits non-zero with one stderr line naming the variable and maps no window, and an `api_key` line in the config file is rejected as an unknown key
- [x] 7.4 Build the request body (turns in order, `stream: true`, resolved model) and the curl configuration (URL, `Authorization: Bearer`, content type, `--connect-timeout 10`, `--speed-limit 1 --speed-time 30`, `--max-time 300`) — verify: a test asserts the configuration contains no `argv` material and that the assembled body matches an expected fixture
- [x] 7.5 Escape the body for both layers (JSON string, then curl config value) — verify: tests cover quotes, backslashes, newlines, CJK and a 64 KiB body, and assert control characters are rejected
- [x] 7.6 Refuse to send when the question is empty or whitespace only, and refuse a second request while one is in flight — verify: tests for `Enter` on spaces and `Enter` during an open stream

## 8. Sandbox

- [x] 8.1 Build the `bwrap` argument table exactly as designed — `--clearenv`, the loader symlinks, the DNS and CA binds — probing that `/usr` and the symlink targets resolve, and fail loudly with no unsandboxed fallback when the sandbox cannot start — verify: a test asserts no code path spawns `curl` without `bwrap`, and a run with `bwrap` hidden from `PATH` reports the failure and sends nothing
- [x] 8.2 Pass the curl configuration on stdin, with the child's stdout already being read by the feed thread — verify: test `a_large_configuration_survives_a_slow_child` writes 100 KiB to a sandboxed `cat` and reads it back whole (the two-pipe deadlock cannot happen: writing finishes before any stdout is consumed, and stdout is drained by another thread)
- [x] 8.3 Assert the credential is absent from every `argv` in the process tree of a live run — verify: during a real request, `ps -ef` and `/proc/*/cmdline` are searched for the key and contain nothing; the result is recorded in the run's notes
- [x] 8.4 Assert the containment: the environment empty, home and the runtime directory absent from inside, the Wayland socket openable from inside only if bound, no writable path, and a sandboxed HTTPS request succeeding against the system trust store — verify: five shell checks run inside the sandbox and their output is recorded
- [ ] 8.5 Kill the child on `Esc` and on exit so no `curl` or `bwrap` survives — verify: after cancelling a stream and closing, the process table holds neither

## 9. Stream reader and failure mapping

- [x] 9.1 Implement the SSE state machine: byte buffering across reads, `\n` and `\r\n`, `data:` payloads only, frames without content ignored, `[DONE]` ends the answer, closing without it means truncated — verify: tests built from the recorded live stream, including a delta split across two reads, a bare-LF stream, a CRLF stream, the role-only opening delta and the trailing `sla_metrics` chunk
- [x] 9.2 Cap a single answer at 64 KiB and mark the truncation in the transcript — verify: a test streams 100 KiB and asserts the retained text is exactly 64 KiB and one truncation entry is present
- [x] 9.3 Map outcomes onto distinct messages: 401 naming the environment variable, 429 naming the provider, 5xx, connection failure, and curl exit 28 split by whether any byte arrived (none: the provider sent nothing; some: the stream stopped) — verify: tests for each status and both exit-28 cases, and an assertion that no message contains a substring of the key

## 10. Transcript model

- [x] 10.1 Order turns, mark a cancelled or truncated answer as incomplete, and distinguish a failure entry from answer text — verify: tests assert entry order, the incomplete marker on cancel and on truncation, and that an incomplete answer stays visible while being left out of the next request body
- [x] 10.2 Assemble the request from this run's turns only, with nothing carried across processes — verify: a test asserts the second request contains both turns in order and that a new process starts empty
- [x] 10.3 Keep the viewport where the user put it while text arrives — verify: a test scrolls up during an append and asserts the first visible row is unchanged

## 11. Headless preview and the no-disk audit

- [x] 11.1 Add `--preview <file.ppm>` rendering a scripted transcript through the real layout and paint code into an offscreen buffer — verify: the mode writes a PPM that opens, and shows the empty state, a streamed answer, a highlighted row and a failure entry
- [ ] 11.2 Use the preview to compare tokens, paddings, radius and row height against a `wlmenu` frame — verify: the two images are put side by side and every token table value is confirmed, not judged by eye
- [ ] 11.3 Audit a run for disk writes — verify: `strace` is not installed on this machine, so the audit was done two other ways and both are weaker than a syscall trace: the complete inventory of path-opening calls (11.4, in the README) and a scratch-directory run (all four XDG directories pointed at an empty directory, which is still empty afterwards). A syscall trace still needs doing on a machine that has `strace`
- [x] 11.4 Review every path-opening call in the source for a write mode — verify: the inventory is written into the README's privacy section

## 12. Documentation

- [x] 12.1 Write the security section: what the sandbox covers, the unrestricted network, the same-user credential exposure, and that the panel itself is not sandboxed — verify: each sentence maps to a check in group 8
- [x] 12.2 Write the privacy section: no disk, no history, the screen and same-user process and swap outside the guarantee, and the clipboard as the deliberate exit — verify: each claim maps to a check in group 11
- [x] 12.3 Copy `wlmenu`'s man page format into `docs/aside.1`, covering flags, keys, environment and the compositor keybinding snippet — verify: `man ./docs/aside.1` renders and the keys match the spec table
- [x] 12.4 Write the manual checklist for a live session: fcitx/ibus typing, 1x/1.5x/2x, resizing the window, `Alt+C`, second launch raising, `Esc` cancelling mid-stream — verify: every item is a step the user can perform and report back on

## 13. Integration checks

- [x] 13.1 Run the full gate and record the output — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` all clean
- [ ] 13.2 Run one real exchange end to end with the default model and record it — verify: the answer streams visibly, the transcript keeps a follow-up turn, and the copy path puts the highlighted line on the clipboard
- [ ] 13.3 Work the live checklist on the user's session and record the outcome — verify: every item in 12.4 has a result, and anything that failed is reported rather than reworded

## 14. Parse fonts with skrifa instead of ttf-parser

- [x] 14.1 Drop `ab_glyph` for `skrifa` (outlines, cmap, metrics) and `ab_glyph_rasterizer` (coverage) — verify: `Cargo.lock` loses `ab_glyph`, `owned_ttf_parser` and `ttf-parser` and keeps `ab_glyph_rasterizer`, and the README's dependency count and list match `cargo tree --edges normal --prefix none | wc -l`
- [x] 14.2 Keep the chain, the per-face scale (`size / (ascender - descender)`) and `ttf-parser`'s point bounds, so no glyph moves — verify: painted frames and per-glyph coverage planes hash identically to the `ab_glyph` build for Latin, CJK through the CFF face, punctuation and four subpixel positions
- [x] 14.3 Memoise `char` to (face, glyph id, advance) — verify: in release `measure` and a warm frame are no slower than the `ab_glyph` build, and a test asserts the memo answers what a re-walked chain answers
- [x] 14.4 Update the places that named the parser — verify: `README.md` and `CLAUDE.md` name `skrifa`, and the gate (`cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings`) is clean
- [x] 14.5 Cap the lookup memo at `LOOKUP_CACHE_CAP`, since its keys come from the provider's text — verify: a test fills it past the cap and asserts it stays bounded; the same test fails with the cap removed (16384 entries)

## 15. Route the request through a proxy the user configures

- [x] 15.1 Resolve `proxy` from the same four sources as the model and base URL (`--proxy`, `ASIDE_PROXY`, the config key), with no default — verify: the spec delta names the four sources, and a test asserts a bare `Config` has no proxy
- [x] 15.2 Write it into the curl configuration as one `proxy = "…"` line, escaped like the URL and the key, and only when set — verify: a test asserts the line is present for a `socks5h://` value and absent (with no occurrence of the word `proxy`) when direct
- [x] 15.3 Name the failing hop when the proxy is the problem — verify: tests for curl exit 5 and 97 assert the message contains `proxy` and not `provider`
- [x] 15.4 Document it: `--proxy` in the usage text and the man page, `ASIDE_PROXY` in the environment list, the config key in `FILES`, and the intercepting-proxy caveat in the README's security section — verify: the four documents name it, and the gate (`cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings`) is clean
- [x] 15.5 Report curl's own diagnosis for a proxy failure instead of guessing, and strip `user:pass@` from any URL before an error is shown — verify: tests cover the unresolvable, malformed-address, unsupported-scheme and refused-tunnel cases, assert a quoted proxy credential never reaches the message while its host does, and assert exit 7 without a proxy still names the provider
- [x] 15.6 Blame the right hop for a connect-phase timeout — verify: `curl` prints identical words for a proxy that never answers CONNECT, a tunnel that came up but never finished TLS, and an unreachable provider address (measured), so `Request` carries `via_proxy`; a test asserts the same 28 text names the proxy through a proxy and the provider without one, and that a stall is never blamed on the proxy
- [x] 15.7 Take the hop from the configuration on every other failure path too — verify: tests assert a stalled provider (`Operation too slow`, measured wording) names the provider with and without a proxy, that a provider host reading `proxy` is not mistaken for the proxy hop, that DNS, certificate and HTTP-status failures stay the provider's through a proxy, that `407` is the proxy's refusal when one is configured, and that curl's proxy codes fall through to curl's own words when no proxy is configured
- [x] 15.8 Confirm the reverse direction end to end, and quote the deadline that actually fired — verify: with a proxy that accepts and never answers and a request that went out, `Request` returns a failure naming the provider and not the proxy, and the message quotes `SPEED_TIME_S` (the stall deadline) rather than the connect deadline, matching curl's own `transferred the last 30 seconds`
