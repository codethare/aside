# Proposal

## Why

The panel answers in whatever the model's default voice is. There is no way to say
"answer in Chinese", "three sentences, no preamble", "you are my terminal's second
pair of eyes" — the constraints a reader would put in an `AGENTS.md` for a coding
agent. Every question has to carry them again, and a question that carries them is a
question the transcript shows and the provider's cache re-reads.

## What Changes

- **A new configuration key, `instructions`.** One or more lines in
  `~/.config/aside/config.toml`; the values are joined in file order with a newline, so
  a short document of rules is written the way the file already reads. It is one
  value, not a path: the panel still reads exactly one file.
- **They travel as the first message of every request**, `role: "system"`, before the
  question and its follow-ups, so a follow-up is under the same constraints as the
  question that started the exchange.
- **Nothing is sent when the key is absent.** The body is byte-for-byte what it was,
  so an unchanged configuration makes an unchanged request.
- **An empty value, or more than 2000 characters in total, is refused** with the file
  and the line — a silent no-op and a silently truncated set of rules are both worse
  than not starting.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `provider-stream`: the configuration can carry instructions for the model, and they
  are sent as the request's leading `system` message.

## Impact

- `src/provider.rs`: the key, its parse-time checks, and the body's leading message.
- `src/chat.rs`: one argument when the body is built.
- `README.md`, `docs/aside.1`, `docs/live-checklist.md`.
- No new dependency, no new file read, nothing on disk.
