## MODIFIED Requirements

### Requirement: The provider request uses the OpenAI-compatible wire

`aside` SHALL send `POST <base>/chat/completions` with `"stream": true`, the
resolved model, and the conversation turns in order, authenticated with an
`Authorization: Bearer` header. Every request SHALL carry a stable
`x-opencode-session` header identifying the conversation, so the provider can route
it and cache its prompt: one value per panel process, reused for every turn, and a
new value for the next summon. That value MUST NOT be derived from the question or
from the transcript. It MUST NOT be empty. It rides in the curl configuration on the
child's stdin, like the credential, so it appears in no `argv`.
It SHALL identify itself as a client in the `User-Agent` header, naming itself and its
version, rather than sending a generic HTTP-library name.
When the configuration carries instructions for the model, the body SHALL begin with
one message of role `system` holding them, before the conversation's first turn, on
every request (see the instructions requirement below).
It MUST NOT send the Anthropic `/messages` wire
in this change, and MUST NOT fall back to requesting `/models`.

#### Scenario: Request shape

- **WHEN** the resolved model is `deepseek/deepseek-v4.1-flash` and one question has been asked
- **THEN** the body contains one user message with that text, `stream` is true, and the reply is streamed rather than awaited whole

#### Scenario: Routing key

- **WHEN** a question and then a follow-up are sent from one panel
- **THEN** both requests carry the same non-empty `x-opencode-session` value, and the next summon carries a different one

#### Scenario: The client names itself

- **WHEN** any request is sent
- **THEN** it carries a `User-Agent` of the form `aside/<version>`, not curl's default

## ADDED Requirements

### Requirement: The configuration carries instructions for the model

The configuration SHALL accept an `instructions` key. It MAY appear more than once,
and the values SHALL be joined in file order with a single newline between them, so a
set of rules is written one rule a line. No other key SHALL accept a repeat.

The joined text SHALL be sent as the leading message of `messages`, with role
`system`, on every request that panel makes — the question and each follow-up alike —
and it MUST NOT be sent anywhere else: not in a header, not in an `argv`, not as a
turn of the conversation. With the key absent, the body SHALL be exactly the body the
panel sent before this key existed.

An `instructions` whose text is empty or only whitespace SHALL be refused, and so SHALL
a total longer than 2000 characters; either is reported with the file and the line and
the panel MUST NOT open. The instructions SHALL NOT be drawn in the panel.

#### Scenario: The rules lead every request

- **WHEN** the configuration sets `instructions` and the user asks a question and then a follow-up
- **THEN** both bodies begin with one `system` message holding those words, before the turns

#### Scenario: Several lines are one value

- **WHEN** `instructions` is written twice, the second line after the first
- **THEN** the `system` message holds the first line, a newline, and the second

#### Scenario: Unset means unchanged

- **WHEN** the configuration has no `instructions`
- **THEN** the body is the same bytes as without this key, with no `system` message

#### Scenario: An empty value refuses to start

- **WHEN** the configuration has `instructions = ""` or only spaces
- **THEN** the panel writes one line naming the file and the line, and exits non-zero without opening

#### Scenario: Too long is refused, not cut

- **WHEN** the instructions add up to more than 2000 characters
- **THEN** the panel reports the file and the line of the value that crossed the limit and does not open

#### Scenario: The panel does not show them

- **WHEN** the instructions are set and an answer arrives
- **THEN** the transcript holds the model's words only, and no part of the instructions
