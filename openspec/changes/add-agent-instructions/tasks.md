# Tasks

## 1. The key

- [x] 1.1 `parse_config` accepts `instructions`, once or repeated, and refuses an empty one and a total that crosses 2000 characters at the line that crossed it — verify: tests assert one line is accepted, a second `instructions` line is kept as its own entry, `""` and `"   "` are refused with their line, and a value whose addition passes the cap is refused at that line
- [x] 1.2 `resolve` joins the lines in file order with a newline into `Config::instructions` — verify: a test resolves a file with two `instructions` lines and reads the two joined by one `\n`, and a file without the key resolves to `None`
- [x] 1.3 No other key starts accepting a repeat — verify: resolving a file with two `model` lines still reads the first

## 2. The request

- [x] 2.1 `request_body(turns, model, instructions)` puts one `system` message before the turns, and no `system` message at all when there are none — verify: a test asserts the body begins `{"model":…,"messages":[{"role":"system",…},` and that the body for `None` is the exact string the panel sent before this key existed
- [x] 2.2 The instructions are escaped like any other content — verify: a test with a quote and a newline in the instructions finds `\"` and `\n` in the body and no raw newline
- [x] 2.3 `Request::start` passes the configured instructions — verify: `request_body` is the only caller and the compiler holds the two together

## 3. Docs

- [x] 3.1 `README.md` (seven keys and an example line), `docs/aside.1` (the key, the repeat rule, the cap), `docs/live-checklist.md` (set it, ask, see the voice change) — verify: each names `instructions`, says the lines join, and says the cap is refused rather than cut

## 4. Integration

- [ ] 4.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean. Waiting on a machine with the system faces installed: this sandbox has no monospace face and no CJK face, so four font-metric tests fail before and after the change, and they are the only failures
- [ ] 4.2 Run a real session — verify: with `instructions = "Answer in Chinese, at most three sentences."` both a question and a follow-up answer in Chinese within three sentences, an empty value refuses to start naming the line, and the panel draws none of it
