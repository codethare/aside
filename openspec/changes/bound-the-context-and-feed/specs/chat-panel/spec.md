## ADDED Requirements

### Requirement: The request history is bounded

One request SHALL carry at most 512 KiB of turn text. When the next question would
pass that, the panel SHALL drop the oldest turns from the request and free their
text, keeping the newest question and the turn in flight untouched. The transcript
SHALL say that earlier turns were left out, once the exchange ends (or at once when
the request never starts, and on a cancel), rather than forgetting them silently.

#### Scenario: A long session stays bounded

- **WHEN** the completed turns hold more than 512 KiB and a new question is sent
- **THEN** the request carries the newest turns up to the cap and the newest question, and the oldest turns' text is gone from memory

#### Scenario: The newest question is never dropped

- **WHEN** the history is over the cap and the panel builds the request
- **THEN** the question just asked is in the body even if it alone is close to the cap

#### Scenario: The cut is visible

- **WHEN** older turns were dropped for a request
- **THEN** a note says earlier turns were left out and names the cap, and the answer to the new question still arrives

## MODIFIED Requirements

### Requirement: The accumulated reply is bounded

The panel SHALL cap the accumulated reply text at 64 KiB and MUST tell the user
that the reply was cut off at the cap, rather than silently truncating it. The
cap MUST NOT discard earlier complete turns. Once the cap is reached, further
deltas SHALL NOT lay the answer out again or request a frame; the stream is still
read to its sentinel so the ending is known.

#### Scenario: A runaway reply hits the cap

- **WHEN** a provider streams more than 64 KiB for one answer
- **THEN** the panel shows the first 64 KiB and one line saying the reply was truncated at the cap

#### Scenario: The rest of a capped stream costs nothing

- **WHEN** deltas keep arriving after the cap is marked
- **THEN** each one reports no change, no row is rebuilt, and no frame is drawn until the stream ends
