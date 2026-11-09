# Route the request with a session id
## Why
Every request to opencode Go comes back `400` before authentication:

```
Request is missing x-opencode-session and cannot be routed efficiently.
```

The endpoint wants a stable id per conversation in the `x-opencode-session`
header, and says why: routing and prompt caching are keyed on it. The panel sends
none, so no credential can help — the refusal happens above the credential.

Measured against `https://opencode.ai/zen/go/v1/chat/completions`:

| headers sent | answer |
|---|---|
| none | `400 MissingSessionID` |
| `x-opencode-session: abc` | `401 authentication_error: Go requires a credential` |
| `x-opencode-session: aside-1` | same |

Any non-empty value clears the routing gate, so the value is ours to choose; the
validated client on this machine sends a plain word too, not a UUID.

The same page asks every client to say who it is: "Identify itself with its own user
agent, such as `my-coding-agent/1.0`, rather than a generic SDK or HTTP-library
name". The panel sends no `User-Agent` of its own, so curl's name goes out and the
traffic looks like a bare library call.
## What changes
- Every request carries `x-opencode-session: <id>`, one id per panel process. A
  panel is one conversation from summon to exit, so the id is made once and reused
  for every follow-up; the next summon is a new conversation and a new id.
- The id is `aside-<pid>-<start-nanos>`: two numbers the process already has,
  joined. No dependency, no randomness from outside, nothing on disk, and nothing
  derived from the question or the transcript.
- Every request says `User-Agent: aside/<version>`, the version being the one in
  `Cargo.toml`, written by the build rather than by hand.
- Nothing else about the request moves: same URL, same body, same
  `Authorization: Bearer`, and both additions ride in the curl configuration on the
  child's stdin, so they are still absent from `argv`.
## Cost
Two more lines in the curl configuration. A conversation cannot be resumed in
another process and keep its routing key, which is what the panel already promises:
the transcript dies with the process.
