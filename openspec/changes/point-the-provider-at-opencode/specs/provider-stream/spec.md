# Spec Delta
## MODIFIED Requirements
### Requirement: Configuration precedence and defaults
`aside` SHALL resolve provider, base URL, model and key from four sources in this
order: command-line flag, environment variable, configuration file, built-in
default. A higher-precedence source MUST override a lower one entirely, with no
merging. The built-in default SHALL be provider `opencode`, base URL
`https://opencode.ai/zen/go/v1`, model `deepseek/deepseek-v4.1-flash`,
and no proxy.
#### Scenario: Environment overrides the config file
- **WHEN** the config file names one model and the environment names another
- **THEN** the request carries the model from the environment
#### Scenario: Default model on a bare machine
- **WHEN** nothing is configured and only `OPENCODE_API_KEY` is set
- **THEN** the request goes to the opencode base URL with model `deepseek/deepseek-v4.1-flash`

### Requirement: The credential comes from the provider's environment variable
`aside` SHALL read the key from `OPENCODE_API_KEY`, and only from there. When
no key is available, `aside` MUST NOT open a connection and MUST name the variable
it looked for. A key MUST NOT appear in any
request header echoed to the panel, log line, error message or rendered output.
For the same reason, any URL an error message quotes SHALL have its `user:pass@`
part removed before it is shown.
#### Scenario: Key never reaches the display
- **WHEN** any request fails for any reason
- **THEN** the rendered error text contains no substring of the key
#### Scenario: The old name is not a second source
- **WHEN** `OPENCODE_API_KEY` is unset and only `COMMAND_CODE_API_KEY` is set
- **THEN** `aside` names `OPENCODE_API_KEY` on stderr, exits non-zero and opens no connection
