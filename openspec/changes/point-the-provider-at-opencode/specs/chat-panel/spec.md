# Spec Delta
## MODIFIED Requirements
### Requirement: The credential is checked before the window opens
When no credential is available for the resolved provider, `aside` MUST write one
line to stderr naming the missing environment variable and exit non-zero, without
creating a window and without opening a connection.
#### Scenario: Missing key
- **WHEN** `OPENCODE_API_KEY` is unset and no key is configured
- **THEN** stderr says which variable is missing, the exit status is non-zero, and no window is mapped
