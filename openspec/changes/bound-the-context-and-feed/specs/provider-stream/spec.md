## ADDED Requirements

### Requirement: The response queue is bounded

The background reader SHALL stop reading when 1 MiB of response bytes are waiting
for the panel, and SHALL resume when they are drained, so the pipe pushes back on
the child instead of the process growing with a stream the panel is not drawing.
When a request finishes or is cancelled, the panel SHALL close the reader, wait for
it, and decode whatever it queued after the last frame's poll, so a child that
exits between two polls does not lose the tail of its stream.

#### Scenario: A stream the panel is not draining

- **WHEN** the provider sends faster than the panel polls and 1 MiB is queued
- **THEN** the reader waits rather than growing the queue, and continues when the next poll drains it

#### Scenario: The child exits between polls

- **WHEN** the child ends and `finish` is called after the last poll, with bytes still queued
- **THEN** those bytes are decoded before the ending is decided, so the last delta and `[DONE]` are not lost

#### Scenario: A cancelled request ends its reader

- **WHEN** `Esc` cancels a request whose reader is waiting with a full queue
- **THEN** the child is killed, the reader is closed and joined, and no thread is left waiting
