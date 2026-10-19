# Spec Delta

## Purpose

The keyboard-first chat window: summoning it, composing a question in it,
conversing with the model across turns, scrolling the transcript, and taking a
line out of it.

## ADDED Requirements

### Requirement: The panel is an ordinary window, summoned by a keybinding

`aside` SHALL present itself as an `xdg-shell` toplevel, not a layer-shell
overlay: movable, resizable, and surviving focus loss. It MUST take keyboard
focus on appearance and MUST NOT be anchored to an output edge. When the
compositor closes the window, `aside` MUST exit rather than continue without a
way to be seen.

#### Scenario: Panel appears focused

- **WHEN** the user runs `aside` from a keybinding on a compositor supporting `xdg-shell`
- **THEN** a window is mapped and the next keystroke is delivered to it, before any provider request is sent

#### Scenario: Window closed by the compositor

- **WHEN** the compositor closes the window while `aside` holds it
- **THEN** the process exits non-zero and writes one line to stderr, rather than idling invisibly

### Requirement: A second launch raises the running panel

Launching `aside` while an instance is already running SHALL raise and focus that
instance and exit zero. It MUST NOT start a second window and MUST NOT dismiss
the running one.

#### Scenario: Second launch raises instead of duplicating

- **WHEN** `aside` is running and the user presses the summoning key again
- **THEN** the existing panel is raised and focused, and no second window appears

### Requirement: The keyboard contract

The panel SHALL give these keys these meanings, and nothing else:

| Key | Action |
|---|---|
| `Enter` | send the typed question |
| `Esc` | cancel an open stream; close the panel when no stream is open |
| `Up` / `Down` | move the transcript highlight one line, scrolling the viewport to keep it visible |
| `PageUp` / `PageDown` | move the transcript highlight one page |
| `Alt+C` | copy the highlighted transcript line to the clipboard |

The input SHALL be one logical line that soft-wraps, edited with the standard
keys — `Left`, `Right`, `Home`, `End`, `Backspace`, `Delete`, and
`Ctrl+Left`/`Ctrl+Right` for word movement. There SHALL be no newline key in this
change: `Enter` always sends, and a multi-line question can still be pasted.

Every action SHALL be reachable without a pointer. No key SHALL have two meanings
in the same state, and `Up` and `Down` SHALL always mean the transcript highlight,
never cursor movement in the input.

#### Scenario: Escape during a stream cancels rather than closes

- **WHEN** the user presses `Esc` while deltas are still arriving
- **THEN** the stream is cancelled, the panel stays open, and the partial reply remains visible and marked as cancelled

#### Scenario: Escape when idle closes

- **WHEN** the user presses `Esc` with no stream open
- **THEN** the process exits

#### Scenario: Enter sends

- **WHEN** the input contains `why` and the user presses `Enter`
- **THEN** a request is sent with `why` as the newest turn, and the input is cleared

### Requirement: Typed input accepts IME-composed text

The panel SHALL act as a `zwp_text_input_v3` client so that a running input method
(fcitx, ibus) can deliver composed text, including multi-character and CJK
commits, pre-edit strings and cursor position. When the compositor advertises no
text-input manager, the panel MUST still accept directly typed keys and MUST
state that input-method support is unavailable, rather than failing to start.

#### Scenario: A composed Chinese question arrives intact

- **WHEN** the user composes `什么是 EAGAIN` with an IME and commits it
- **THEN** the input line holds exactly that text and pre-edit text is rendered while it is being composed

#### Scenario: No input method on the compositor

- **WHEN** `zwp_text_input_manager_v3` is not advertised
- **THEN** the panel opens, accepts directly typed characters, and reports that IME input is unavailable

### Requirement: Turns accumulate in one transcript

The panel SHALL keep every turn of the current run in one ordered transcript, and
sending a question MUST keep all earlier turns on screen. Each request MUST carry
the preceding turns of this run in order, and MUST NOT carry anything from a
previous run of the process. An answer that was cancelled or cut short SHALL stay
visible in the transcript, and MUST be left out of subsequent requests rather than
sent back to the provider as if it were complete.

#### Scenario: A follow-up keeps the first answer

- **WHEN** the user asks a question, receives an answer, and asks a follow-up
- **THEN** the transcript shows both questions and both answers, and the request body contains the first turn followed by the follow-up

#### Scenario: Nothing survives a restart

- **WHEN** `aside` is closed and started again
- **THEN** the transcript is empty and the request contains only the new question

### Requirement: The answer streams in as it arrives

The panel SHALL append each decoded text delta as it arrives, so the visible
answer grows while the stream is open. It MUST NOT wait for completion before
showing anything, and the panel MUST remain responsive — accepting keystrokes and
repainting — throughout.

#### Scenario: Partial text visible before completion

- **WHEN** the first content delta has arrived and the stream is still open
- **THEN** that text is displayed and the working indicator is shown

#### Scenario: The UI does not block on the network

- **WHEN** a stream is open and the provider has paused
- **THEN** the panel still responds to `Esc` and scrolls immediately

### Requirement: The working indicator never moves the layout

While a stream is open the panel SHALL show a working indication that occupies no
space and changes no geometry, and the indication MUST disappear when the stream
ends without the transcript shifting.

#### Scenario: Indicator disappears without a jump

- **WHEN** a stream ends
- **THEN** the rendered position of every transcript line is unchanged from the frame before the stream ended

### Requirement: The transcript scrolls with flat frame cost

The panel SHALL show a bounded viewport of at most 64 text rows and support
scrolling through a transcript far larger than that. Frame cost MUST stay
proportional to the number of visible rows and MUST NOT grow with the size of the
transcript; re-laying out the whole transcript on every frame is not acceptable.

#### Scenario: Large transcript, steady frame

- **WHEN** the transcript holds tens of thousands of characters and a stream is appending to it
- **THEN** the per-frame layout work stays proportional to the visible rows

#### Scenario: Scrolling back during a stream

- **WHEN** the user scrolls up while an answer is streaming
- **THEN** the viewport stays where the user put it and does not snap back to the tail

### Requirement: The accumulated reply is bounded

The panel SHALL cap the accumulated reply text at 64 KiB and MUST tell the user
that the reply was cut off at the cap, rather than silently truncating it. The
cap MUST NOT discard earlier complete turns.

#### Scenario: A runaway reply hits the cap

- **WHEN** a provider streams more than 64 KiB for one answer
- **THEN** the panel shows the first 64 KiB and one line saying the reply was truncated at the cap

### Requirement: A line can be copied to the clipboard

`Alt+C` SHALL copy the highlighted transcript line, and only that line, to the
clipboard by running `wl-copy`. The copy MUST be a deliberate user action: the
panel MUST NOT set the clipboard on its own, at any other time, or on exit.

#### Scenario: Copying the selected line

- **WHEN** the user highlights a line and presses `Alt+C`
- **THEN** the clipboard holds exactly that line's text, and does not hold the rest of the transcript

#### Scenario: No implicit copy

- **WHEN** a reply finishes and the user closes the panel without pressing `Alt+C`
- **THEN** neither the clipboard nor the primary selection has been set by `aside`

### Requirement: Failures and empty states are part of the transcript

A provider or transport failure SHALL be rendered as its own transcript entry
distinct from answer text, naming the provider and the cause, and the panel MUST
remain interactive and closable in every failure state. An empty transcript MUST
be distinguishable from a pending one.

#### Scenario: Rejected credential

- **WHEN** the provider answers `401`
- **THEN** the transcript shows one entry naming the provider and the environment variable that should hold its key, and `Esc` still closes the panel

#### Scenario: Empty state

- **WHEN** the panel opens and nothing has been asked yet
- **THEN** the input line is focused and the transcript area is drawn as an empty state, not as a blank pending answer

### Requirement: The credential is checked before the window opens

When no credential is available for the resolved provider, `aside` MUST write one
line to stderr naming the missing environment variable and exit non-zero, without
creating a window and without opening a connection.

#### Scenario: Missing key

- **WHEN** `COMMAND_CODE_API_KEY` is unset and no key is configured
- **THEN** stderr says which variable is missing, the exit status is non-zero, and no window is mapped

### Requirement: The window can be resized and re-wraps once per resize

The panel SHALL honour the size the compositor configures, in both dimensions,
wherever the user drags it. The transcript SHALL be re-wrapped when the width
changes, and that re-wrap MUST happen once per resize rather than on every frame.
The visible transcript SHALL remain bounded when the window is made very tall.

#### Scenario: Resizing re-wraps the transcript

- **WHEN** the user drags the window narrower while the transcript holds several answers
- **THEN** every row is re-wrapped to the new width and the frame cost returns to the per-visible-row cost on the next frame

#### Scenario: A very tall window

- **WHEN** the window is made taller than 64 text rows
- **THEN** the panel renders at most 64 rows and scrolls the rest

### Requirement: The panel is correct at 1x, 1.5x and 2x

The panel SHALL honour `wl_output.scale`, INCLUDING the case where the display's
scale changes while it is open, and the first commit of a surface MUST match the
size the compositor configured. Subpixel text filtering SHALL be used only at
scale 1, where it is legible, and grayscale antialiasing at higher scales.

#### Scenario: Fractional and integer scaling

- **WHEN** the panel is mapped on a 1x, 1.5x and 2x output in turn
- **THEN** text is the same logical size on all three and no colour fringing appears above scale 1

#### Scenario: A protocol error never becomes a spin

- **WHEN** the compositor reports a protocol error, including a first-commit size mismatch
- **THEN** `aside` reports it and exits, and no failure path leaves the process burning CPU

#### Scenario: The compositor closes the connection

- **WHEN** the compositor goes away while a panel is on screen, so the socket reports a broken pipe
- **THEN** `aside` exits quietly with status 0, because the session ending is not a failure of the panel
- **AND** every other dispatch failure, and any protocol error, is still reported and exits non-zero

#### Scenario: The connection closes before the panel appeared

- **WHEN** the socket reports a closed connection and no frame has reached the compositor yet
- **THEN** `aside` reports that the compositor closed the connection before the first frame and exits non-zero, because nothing was ever drawn and silence would leave no trace
- **AND** when the compositor sent an error of its own first, that error is what is reported, since it is the reason

### Requirement: A run with no transcript reaches the first frame quickly

The first frame SHALL be drawn without waiting for the credential check, font
loading of the fallback chain, or any provider catalog, and font loading SHALL
overlap the Wayland handshake.

#### Scenario: Cold start with no transcript

- **WHEN** `aside` is started on an idle session
- **THEN** the first frame is presented well under 50 ms after the connection is established
