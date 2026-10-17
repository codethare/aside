# Spec Delta

## Purpose

The promise that makes `aside` usable with sensitive text: the question and the
answer exist in this machine's memory for the lifetime of one process, and
nowhere else.

## ADDED Requirements

### Requirement: No conversation content is written to disk

During a run `aside` MUST NOT open any file for writing, creating or appending,
with the sole exception of the standard output and standard error descriptors
inherited from its parent. The question, the assembled request body, the raw
stream bytes and the rendered answer SHALL live in memory only, and the transport
MUST NOT spool a request or response body to a temporary file.

#### Scenario: A full run opens nothing for writing

- **WHEN** a run completes against a provider
- **THEN** tracing the process tree shows no `openat` with write, read-write or create flags, and no temporary file is created

#### Scenario: A reply larger than any internal buffer

- **WHEN** the provider's reply exceeds the panel's reply cap
- **THEN** the discarded remainder is dropped in memory and no file has been created

### Requirement: Nothing survives the process

`aside` MUST NOT create or read a state, cache or history directory, and MUST NOT
persist the question, the answer or any derived value. A later run MUST NOT be
able to recover any part of an earlier one. No child process SHALL be left
holding the conversation after the panel exits, other than a clipboard that the
user explicitly asked for.

#### Scenario: Two runs share nothing

- **WHEN** `aside` is run, closed, and run again
- **THEN** the second run's transcript is empty and nothing under the config, cache, state or data directories has been created or modified

#### Scenario: The transcript dies with the process

- **WHEN** the panel exits
- **THEN** the only surviving copy of the answer is the clipboard if `Alt+C` was pressed, and nothing else

### Requirement: Diagnostics never reach a file and never carry content

`aside` SHALL write diagnostics to standard error only. It MUST NOT install a
file appender or enable HTTP body or header tracing at any verbosity, and MUST
NOT include the question, the answer, any part of either, or the API key in any
diagnostic line. Whether the parent redirects standard error into a journal is
outside `aside`'s control and SHALL be stated as such.

#### Scenario: Verbose output contains no content

- **WHEN** `aside` runs with its most verbose diagnostic setting and completes an exchange
- **THEN** no line on standard error contains the question, the answer or the key

### Requirement: The memory claim is stated honestly

The documentation SHALL state the ceiling of the guarantee: the compositor and
anything able to read the screen can see the rendered answer; a process running
as the same user with `CAP_SYS_PTRACE`, and root, can read the memory regardless;
the desktop may swap the memory out; and copying a line to the clipboard is a
deliberate exit from the boundary, after which `wl-copy` serves that line to
other applications. It MUST NOT claim that the process is unrecoverable in memory.

#### Scenario: The ceiling is stated

- **WHEN** a user reads the privacy section of the documentation
- **THEN** the screen, the same-user process, swap and the clipboard are each named as outside the guarantee
