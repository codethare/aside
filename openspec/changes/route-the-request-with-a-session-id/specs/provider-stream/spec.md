# Spec Delta
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
