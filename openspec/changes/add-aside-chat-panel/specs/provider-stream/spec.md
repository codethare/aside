# Spec Delta

## Purpose

Turns a question plus resolved configuration into one streaming request against
the provider, and turns the provider's stream back into an ordered sequence of
text deltas the panel can render.

## ADDED Requirements

### Requirement: Configuration precedence and defaults

`aside` SHALL resolve provider, base URL, model and key from four sources in this
order: command-line flag, environment variable, configuration file, built-in
default. A higher-precedence source MUST override a lower one entirely, with no
merging. The built-in default SHALL be provider `commandcode`, base URL
`https://api.commandcode.ai/provider/v1`, model `deepseek/deepseek-v4.1-flash`,
and no proxy.

#### Scenario: Environment overrides the config file

- **WHEN** the config file names one model and the environment names another
- **THEN** the request carries the model from the environment

#### Scenario: Default model on a bare machine

- **WHEN** nothing is configured and only `COMMAND_CODE_API_KEY` is set
- **THEN** the request goes to the Command Code base URL with model `deepseek/deepseek-v4.1-flash`

### Requirement: The configuration file is read and never written

`aside` SHALL read `$XDG_CONFIG_HOME/aside/config.toml`, falling back to
`~/.config/aside/config.toml`. The file SHALL never be created, written or
appended. Its absence MUST NOT be an error. The file SHALL hold flat `key =
"value"` lines with `#` comments; `model`, `base_url` and `proxy` are the keys
recognised in this change. An unrecognised key, or any line that is not one of
those forms,
MUST produce an error naming the path and the failing line, and MUST NOT be
partially applied. The API key MUST NOT be read from this file: it comes from the
environment only.

#### Scenario: No config file

- **WHEN** the config file does not exist and the values come from the environment
- **THEN** the panel proceeds normally

#### Scenario: Malformed config

- **WHEN** the config file has a syntax error on line 4
- **THEN** `aside` reports the path and line 4 and sends no request

### Requirement: The credential comes from the provider's environment variable

`aside` SHALL read the key from `COMMAND_CODE_API_KEY`, and only from there. When
no key is available, `aside` MUST NOT open a connection and MUST name the variable
it looked for. A key MUST NOT appear in any
request header echoed to the panel, log line, error message or rendered output.
For the same reason, any URL an error message quotes SHALL have its `user:pass@`
part removed before it is shown.

#### Scenario: Key never reaches the display

- **WHEN** any request fails for any reason
- **THEN** the rendered error text contains no substring of the key

### Requirement: The provider request uses the OpenAI-compatible wire

`aside` SHALL send `POST <base>/chat/completions` with `"stream": true`, the
resolved model, and the conversation turns in order, authenticated with an
`Authorization: Bearer` header. It MUST NOT send the Anthropic `/messages` wire
in this change, and MUST NOT fall back to requesting `/models`.

#### Scenario: Request shape

- **WHEN** the resolved model is `deepseek/deepseek-v4.1-flash` and one question has been asked
- **THEN** the body contains one user message with that text, `stream` is true, and the reply is streamed rather than awaited whole

### Requirement: Delta assembly tolerates real provider streams

The reader SHALL emit text in the order sent. It MUST ignore frames carrying no
text, including role-only opening deltas, keep-alive comment frames, and terminal
chunks whose content is absent. It MUST accept line endings of either `\n` or
`\r\n`, MUST reassemble payloads split across reads and a final line without a
trailing newline, and MUST end the answer on the `[DONE]` sentinel. A stream that
closes without `[DONE]` MUST be reported as truncated rather than complete.

#### Scenario: Frames without text produce no text

- **WHEN** the byte stream contains a role-only delta, then `data: ` with an empty delta, then a delta with `"Hello"`
- **THEN** the assembled answer is exactly `Hello`

#### Scenario: A delta split across two reads

- **WHEN** `data: {"choices":[{"delta":{"content":"wor` and `ld"}}]}` arrive in separate reads
- **THEN** the assembled answer contains `world`

#### Scenario: CRLF and bare LF both parse

- **WHEN** the same payloads are delivered with `\r\n` separators and with `\n` separators
- **THEN** both produce identical answer text with no stray carriage returns

#### Scenario: Truncated stream

- **WHEN** the connection closes before `[DONE]`
- **THEN** the partial text stays visible and the entry is marked incomplete

### Requirement: Every failure mode is distinguishable and bounded

`aside` SHALL map outcomes onto distinct, actionable messages: authentication
rejection naming the expected environment variable, rate limiting naming the
provider, provider server error, connection failure, no response within the
connect deadline, and a stream that stopped delivering mid-answer. It MUST apply a
deadline to connecting, a deadline to a stream that stops delivering, and an upper
bound to the whole reply, and MUST NOT wait indefinitely on a stalled connection.
No failure may leave the request open or the panel blocked.

#### Scenario: Rate limited

- **WHEN** the provider answers `429`
- **THEN** the transcript says the provider is rate limiting and names it

#### Scenario: No response at all

- **WHEN** the connection is established but not one byte of the answer arrives before the deadline
- **THEN** the request is abandoned, the transcript reports that the provider sent nothing, and the panel stays usable

#### Scenario: A stream that stops delivering

- **WHEN** several deltas have arrived and then nothing arrives for longer than the stall deadline
- **THEN** the request is abandoned, the partial answer stays visible, and the transcript says the stream stopped

### Requirement: A configured proxy carries the request

`aside` SHALL resolve an optional proxy for the provider request from the same
four sources and in the same order as the model and base URL: `--proxy`,
`ASIDE_PROXY`, the `proxy` key, and no built-in default. The value SHALL be handed
to `curl` as its proxy, so every scheme curl accepts (`http://`, `https://`,
`socks5h://`, …) works and no proxy handling lives in the panel. With no proxy
configured the request MUST go direct: the sandbox runs with an empty environment,
so no `http_proxy`, `https_proxy` or `all_proxy` can reach it. The value MUST NOT
appear in a command line or the process table, MUST NOT be echoed in any error
message, and SHALL be escaped for curl's configuration parser exactly as the URL
and the credential are. Which hop a failure is attributed to SHALL follow from
whether a proxy is configured, never from curl's wording: curl prints the same
words whichever hop was stuck, and a provider's host name may itself read
`proxy`. A proxy that terminates TLS sees the question and the credential, which
is the user's own choice; the README's security section SHALL say so.

#### Scenario: No proxy configured

- **WHEN** no flag, environment variable or configuration key names a proxy
- **THEN** the curl configuration carries no proxy line and the request goes direct

#### Scenario: A SOCKS proxy

- **WHEN** `--proxy socks5h://127.0.0.1:1080` is given
- **THEN** the curl configuration carries that proxy, and nothing about it reaches the process table

#### Scenario: The proxy is unreachable

- **WHEN** the proxy cannot be resolved or refuses the connection
- **THEN** the transcript names the proxy as the failing hop, not the provider

#### Scenario: A proxy address curl will not accept

- **WHEN** the configured proxy is a URL curl refuses as malformed or as an unsupported scheme
- **THEN** the transcript reports curl's own diagnosis of the address, and not that its host name could not be resolved

#### Scenario: The connect deadline passes with a proxy configured

- **WHEN** a proxy is configured and curl reports a connect-phase timeout, so the request was never sent
- **THEN** the transcript says the request never got through the proxy, and does not say the provider sent nothing
- **AND** the same timeout with no proxy configured names the provider instead

#### Scenario: The provider accepts and then says nothing, through a proxy

- **WHEN** the connection is up, the request has been sent, and nothing arrives before the stall deadline
- **THEN** the transcript names the provider, and never the proxy, whether or not one carried the request

#### Scenario: A proxy that refuses with 407

- **WHEN** a request through a configured proxy comes back `407`
- **THEN** the transcript says the proxy refused it, not the provider

### Requirement: An empty question is never sent

`aside` MUST NOT send a request when the typed question is empty or whitespace
only, and MUST leave the input focused.

#### Scenario: Enter on an empty input

- **WHEN** the input holds spaces and the user presses `Enter`
- **THEN** no request is made and the text stays in the input

### Requirement: At most one request is in flight

`aside` SHALL have at most one open stream at a time. Pressing `Enter` while a
stream is open MUST NOT start a second request; the keystroke MUST either be
ignored or, if the input is non-empty, be held in the input until the stream ends.

#### Scenario: Enter during a stream

- **WHEN** a stream is open and the user presses `Enter` with a non-empty input
- **THEN** no second request is opened and the typed text is still in the input afterwards
