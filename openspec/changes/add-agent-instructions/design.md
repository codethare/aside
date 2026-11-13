# Design

## Context

`provider::parse_config` reads flat `key = "value"` lines with `#` comments and refuses
unknown keys, so a typo cannot quietly change nothing; it returns `(String, String)`
pairs and `resolve` turns them into `Config { base_url, model, proxy }`.
`provider::request_body(turns, model)` writes the OpenAI-compatible body:
`{"model":…,"messages":[{"role":"user"|"assistant","content":…}],"stream":true}`. The
credential rides on curl's stdin, never in the body or an `argv`.

## Goals / Non-Goals

- Let a reader state constraints once, in the file they already edit, and have every
  question follow them.
- Non-Goals: a file of instructions (the panel reads exactly one file), per-turn or
  per-mode instructions, editing or previewing them in the panel, or sending them in
  any position other than the head of `messages`.

## Decisions

**1. `instructions` may be written more than once; the values join with a newline, in
file order.** The config is a line list and a set of rules is a paragraph: repeating
the key is how the file already spells one value per line, and it needs no escape
syntax, no multi-line quoting, and no second file. No other key accepts a repeat — a
repeated `model` still resolves to its first line — because that would change how an
existing file is read.
*Alternative rejected:* `instructions_file = "~/rules.md"` — the product reads one
file, refuses to write any, and a path in the configuration is the first step to a
second thing that has to be found, checked and leaked.

**2. One `system` message at the head of `messages`, on every request.** That is the
OpenAI-compatible way to say it, so the same configuration works for any provider the
base URL points at, and the position is the head so no turn can override it.
*Alternative rejected:* a top-level `"system"` field (Anthropic-shaped) — this panel
speaks the OpenAI wire for every provider it supports.

**3. Absent means nothing is sent.** `request_body` with no instructions is the body it
wrote before, byte for byte; a reader who does not set the key cannot have their
request changed by this feature. The zero case is worth a test precisely because it is
the case every existing user is in.

**4. Refused, not truncated, at 2000 characters, and an empty value is an error.**
Both mistakes are silent otherwise: an empty `instructions` would look like it was
applied, and a truncated one would drop exactly the last rule the reader wrote. The
config path already reports `file:line` and exits 2, so the cost of being strict is one
line on stderr.
*Alternative rejected:* trimming to the cap — the panel would send rules the reader
cannot see it dropped.

**5. The panel never displays them.** They are a constraint on the model, not part of
the conversation: drawing them would be a second transcript, and the panel shows only
what the model said. A provider that rejects a `system` message reports it through the
failure path that already exists, with the provider's own words.
