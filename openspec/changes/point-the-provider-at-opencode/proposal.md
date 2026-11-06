# Point the panel at opencode
## Why
The panel's provider changed. Two defaults still name the old platform: the
environment variable a user exports, and the base URL every request goes to. A
launch then needs a rename in front of it —
`OPENCODE_API_KEY="$OPENCODE_API_KEY" aside --base-url …` — and a fresh install
talks to a service the user is not paying for.

Nothing about the credential itself moved: still one key, still read from the
environment only, still written into the curl configuration on the child's stdin
rather than into `argv`. The wire does not move either: opencode serves the
OpenAI chat-completions shape, `/chat/completions` under its base URL, authenticated
with `Authorization: Bearer`.
## What changes
- The variable `aside` reads is `OPENCODE_API_KEY`. The refusal line on stderr, the
  README, the man page and the live checklist all name that one. The old name is
  not read: the spec says the key comes from one variable, and a second accepted one
  would be a second source for a credential, with no rule for which wins.
- The built-in base URL is `https://opencode.ai/zen/go/v1`, so a bare machine with
  only the key exported reaches opencode. Measured, not assumed: `/zen/go/v1/models`
  answers 200 without a credential, and `/zen/go/v1/chat/completions` answers 400 for
  an empty body — the route exists and wants a body, which is what the panel sends.
- The model default stays `deepseek/deepseek-v4.1-flash`: opencode lists
  `deepseek/deepseek-v4.1-flash` as that model's `canonical_id`, so the value a
  commandcode-era config already holds keeps working.
- Nothing else moves. Precedence, the recognised config keys, the sandbox, and the
  absence of the credential from `argv` and from any rendered text stay as they are.
## Cost
A setup that exports the old name stops opening the panel: one line on stderr names
the variable that is missing and the exit status is non-zero, so it is a visible
failure rather than a silent one. The fix is renaming one export. A setup that
already passes `--base-url` or a `base_url` in its config keeps pointing where it
pointed, because both outrank the default.
