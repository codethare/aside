# Spec Delta

## MODIFIED Requirements

### Requirement: The panel is a layer surface in two sizes

The panel SHALL be a `wlr-layer-shell` surface on the overlay layer, anchored to the
bottom and left edges, horizontally centred by its own left margin, with
**on-demand** keyboard interactivity: it SHALL take the keyboard while it is
focused, and it MUST NOT keep it from another window — clicking a window behind it
SHALL focus that window and typing SHALL go there. Its bottom edge SHALL sit 38 % of
the output height above the screen's bottom edge. It SHALL have exactly three
states and two sizes:

| State | Panel size (logical px, 1×) | Content |
|---|---|---|
| idle | 640 × bar_h | the input bar alone |
| thinking | 640 × bar_h | the same bar, with a light travelling its rim |
| answering | 640 × (bar_h + 16 + cap), fixed | the replies above, the bar below |

`bar_h` SHALL be `row_h_for(font_size)`, and `font_size` SHALL come from the
configuration (default 20), so the whole panel follows the configured size.

#### Scenario: Another window can be used

- **WHEN** the panel is open and the user clicks a terminal behind it
- **THEN** the terminal takes the keyboard and typed text goes to the terminal, not to the panel

#### Scenario: The panel is still typable

- **WHEN** the user summons the panel, or clicks it after using another window
- **THEN** it takes the keyboard and typed text reaches its input line

#### Scenario: A larger font

- **WHEN** `font_size` is `30`
- **THEN** the bar is `row_h_for(30)` tall, the text is drawn at 30 px, and the panel is taller by exactly that difference

### Requirement: The keyboard contract

The panel SHALL give these keys these meanings, and nothing else:

| Key | Mode | Action |
|---|---|---|
| `Enter` | typing, vim | send the typed question |
| `Esc` | typing | cancel an open stream; close the panel when no stream is open |
| `Esc` | vim | leave vim mode, cancelling nothing |
| `Esc` | prefix | leave prefix mode, doing nothing else |
| `Ctrl+T` | typing | enter the prefix mode |
| `[` | prefix | enter vim mode |
| `h` / `j` / `k` / `l` | vim | move the **panel** 16 px left / down / up / right |
| `Ctrl+h` / `Ctrl+j` / `Ctrl+k` / `Ctrl+l` | vim | the same directions, 160 px |
| `n` / `p` | vim | move the highlight one page down / up |
| `Ctrl+n` / `Ctrl+p` | vim | move it one line down / up |
| `g` / `G` | vim | move it to the first / last line |
| `Up` / `Down` | typing | move the highlight one line |
| `PageUp` / `PageDown` | typing | move it one page |
| `Alt+C` / `Ctrl+C` | typing, vim | copy the highlighted line |
| `Ctrl+V` | typing | paste the clipboard into the input line |
| any other printable key | vim, prefix | nothing: the input line does not change |

The input SHALL be one logical line that soft-wraps, edited with the standard
keys. There SHALL be no newline key: `Enter` always sends, and a multi-line
question can still be pasted. Every action SHALL be reachable without a pointer. No
key SHALL have two meanings in the same mode, and `Up` and `Down` SHALL always mean
the transcript highlight in typing mode.

#### Scenario: Paste inserts the clipboard

- **WHEN** the clipboard holds `why EAGAIN` and the user presses `Ctrl+V`
- **THEN** the input line receives exactly that text, at the caret

#### Scenario: Copy takes one line

- **WHEN** the user presses `Ctrl+C` with a line highlighted
- **THEN** the clipboard holds that line and nothing else

#### Scenario: Escapes still mean what they meant

- **WHEN** the user presses `Esc` in typing mode while deltas are still arriving
- **THEN** the stream is cancelled, the panel stays open

## ADDED Requirements

### Requirement: The font is configured, not compiled in

`aside` SHALL read `font_size` and `font_family` from the configuration file beside
`model`, `base_url` and `proxy`. `font_size` SHALL be a whole or fractional number
of logical pixels, between 8 and 72 inclusive; anything else SHALL be a
configuration error naming the file and the line, and the process SHALL exit before
opening a window. `font_family` SHALL be a family name matched against the system's
fonts, and a name nothing provides SHALL fall back to the built-in chain rather than
failing to start. Both keys SHALL be optional, and both SHALL apply on the next run
only: the panel never reloads its configuration while it is open.

#### Scenario: A size in the file

- **WHEN** the configuration holds `font_size = "28"`
- **THEN** the panel is drawn at 28 px, the bar is `row_h_for(28)` tall, and CJK still renders through the fallback chain

#### Scenario: A family in the file

- **WHEN** the configuration holds `font_family = "DejaVu Sans Mono"`
- **THEN** every glyph the family carries is drawn by that face

#### Scenario: A family nothing provides

- **WHEN** `font_family` names a font that is not installed
- **THEN** the panel starts with its built-in chain, rather than refusing to run

#### Scenario: A size that is not a size

- **WHEN** `font_size = "huge"` or `font_size = "2"`
- **THEN** one line on stderr names the file, the line and the allowed range, and no window opens
