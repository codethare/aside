# Spec Delta

## Purpose

The containment of the process that talks to the provider: it gets the network
and nothing else, so a question cannot be turned into local access, and a
compromised or misbehaving dependency has nothing of the user's to touch.

## ADDED Requirements

### Requirement: The provider request is made from inside a sandbox

Every request to a provider SHALL be made by `curl` running as a child of `bwrap`.
The panel process itself MUST NOT open a connection to a provider. When the
sandbox cannot be created, `aside` MUST report it and MUST NOT run `curl`
unsandboxed.

#### Scenario: The panel process opens no provider socket

- **WHEN** a question is asked and answered
- **THEN** the connection is owned by the sandboxed `curl` child, not by the panel process

#### Scenario: Sandbox unavailable

- **WHEN** `bwrap` is missing, or user namespaces are unavailable so it cannot start
- **THEN** `aside` writes one line naming the failure, sends no request, and does not fall back to an unsandboxed `curl`

### Requirement: The sandbox exposes the network and the system libraries, nothing else

Inside the sandbox the process SHALL have: the network, the system library tree
and its dynamic loader readable, the name-resolution and certificate files
needed for TLS, and `/proc` and `/dev`. It MUST NOT have the user's home
directory, `$XDG_RUNTIME_DIR`, the Wayland socket, the panel's own files, the
config file, or a writable path, and it SHALL start with an empty environment, so
that no variable the panel inherited — including the credential — is visible
inside.

#### Scenario: The user's files are absent

- **WHEN** a process inside the sandbox lists the home directory or the runtime directory
- **THEN** both are absent

#### Scenario: The Wayland socket is unreachable

- **WHEN** a process inside the sandbox attempts to open the compositor socket named by `$WAYLAND_DISPLAY`
- **THEN** the attempt fails

#### Scenario: The environment is empty

- **WHEN** a process inside the sandbox prints its environment
- **THEN** the output is empty, and in particular holds no copy of the credential and no `HOME`

#### Scenario: Nothing is writable

- **WHEN** a process inside the sandbox attempts to create a file under `/tmp` or anywhere else
- **THEN** the attempt fails or the write lands in a sandbox-private temporary filesystem that disappears with the process

#### Scenario: TLS still works

- **WHEN** a sandboxed `curl` requests a provider endpoint over HTTPS
- **THEN** certificate verification succeeds against the system trust store

### Requirement: The credential never appears in a process argument list

The API key SHALL be handed to the sandboxed `curl` over a file descriptor, and
MUST NOT appear in the `argv` of `bwrap` or `curl`, including as a `bwrap`
`--setenv` value, since those arguments are readable by other local users.

#### Scenario: Another local user reads the command line

- **WHEN** a run is in flight and the process table is read by another user
- **THEN** no argument of any process in the run contains the key

### Requirement: The sandboxed process cannot outlive or signal the panel

The sandbox SHALL start a new session so that it cannot inject input into a
terminal, and SHALL die with the panel. Closing the panel, or cancelling a
stream, MUST leave no `curl` or `bwrap` process behind.

#### Scenario: Closing during a stream

- **WHEN** the user cancels a stream and closes the panel
- **THEN** the sandboxed process is gone and no orphan remains

### Requirement: The limits of the containment are documented

The documentation SHALL state what the sandbox does not cover: the network is
reachable without restriction, so the provider and anything that can MITM the
connection sees the question; the key is readable by any process running as the
same user; the panel itself is not sandboxed; and the sandbox does not make the
provider's answer trustworthy. It MUST NOT claim the sandbox provides egress
filtering or that a question cannot be exfiltrated by a compromised provider.

#### Scenario: A reader can find the ceiling

- **WHEN** a user reads the security section of the documentation
- **THEN** the unrestricted network, the same-user credential exposure and the unsandboxed panel are each stated explicitly
