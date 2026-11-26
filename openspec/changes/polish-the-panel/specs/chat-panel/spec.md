# Spec Delta

## ADDED Requirements

### Requirement: Streaming respects the reader's scroll

Reply text that arrives while the view is at the tail SHALL move the view with it.
Reply text that arrives while the reader has scrolled up SHALL NOT move the view:
the reader keeps the place they chose, and the panel SHALL show a small `new content`
mark at the bottom-left of the replies card until the view is back at the tail. The
end of a request SHALL follow the same rule: the final text and any closing note SHALL
be clamped into view only when the view was already following. Scrolling back to the
tail SHALL clear the mark and resume following.

#### Scenario: Reading an earlier part while the answer streams

- **WHEN** the reader scrolls up during a streaming answer and more text arrives
- **THEN** the view stays where they put it and a `new content` mark appears at the bottom of the replies

#### Scenario: Returning to the tail

- **WHEN** the reader scrolls back to the last line
- **THEN** the mark disappears and later text follows the tail again

#### Scenario: The answer ends while scrolled up

- **WHEN** the request finishes while the reader is above the tail
- **THEN** the view is not yanked to the end and the mark remains until they return

### Requirement: Clipboard and returned-question feedback is visible

A copy SHALL put a short `copied N line(s)` note in the bar. A paste that reaches the
64 KiB cap, either because the clipboard read was bounded or because the line had no
room, SHALL put a note saying the text was cut at the cap in the bar instead of
silently shortening it. A failed request that puts its question back into the input
line SHALL put `question returned to the line` in the bar and light a faint accent
rim on the bar for a short moment; the caret SHALL be at the end of the restored
question. These notes SHALL be transient and SHALL NOT be written anywhere.

#### Scenario: A long paste

- **WHEN** the clipboard holds more than the input cap and `Ctrl+V` is pressed
- **THEN** the line holds the capped text on a whole character and the bar says it was cut

#### Scenario: A failed request

- **WHEN** the provider fails and the line was empty
- **THEN** the question is back in the line, the caret is at its end, and the bar glows and says the question returned

### Requirement: The bar shows focus and waiting

While the panel does not hold the keyboard the bar SHALL draw a very faint inner rim;
the rim SHALL disappear when the compositor gives the panel the keyboard. While a
request is in flight the bar's right-hand side SHALL read `Esc cancel`; after three
seconds it SHALL add the elapsed whole seconds. A toast or a mode label MAY outrank
this hint, but the hint SHALL return when they are gone.

#### Scenario: Keyboard elsewhere

- **WHEN** the panel is on screen but the compositor has given the keyboard to another window
- **THEN** the bar draws its faint unfocused rim and no caret

#### Scenario: A slow request

- **WHEN** a request has been running for four seconds
- **THEN** the bar's right side reads `Esc cancel` and the elapsed seconds

### Requirement: Typing interrupts the leaving

The first character written into the bar while a question is leaving SHALL clear the
question and stop its particles in that frame. The bar SHALL show the new input, not
a mixture of the new line and the old particles.

#### Scenario: Typing during the exit

- **WHEN** the reader types while the question's particles are still in the bar
- **THEN** the particles are gone and the typed character is what the bar shows

### Requirement: Font size and family can be chosen at startup

`--font-size PX` SHALL override `font_size` for the run and `--font-family NAME`
SHALL override `font_family`. A size outside the accepted 8 to 72 range SHALL be
refused with one line on stderr and no panel. `--preview` SHALL use the overridden
size, so the particle density can be compared at another size without editing the
configuration.

#### Scenario: A preview at another size

- **WHEN** `aside --font-size 32 --preview out.ppm` runs
- **THEN** the preview and its leaving frames are drawn at 32 px

### Requirement: The help lists the keys

`--help` SHALL print a compact key table covering sending, cancelling, pasting,
copying, scrolling, the `Ctrl+T` prefix, copy mode, moving and resizing the panel.
The table SHALL be short enough to fit one page with the existing usage text.

#### Scenario: Asking for help

- **WHEN** the user runs `aside --help`
- **THEN** the usage and the key table are printed and the process exits

### Requirement: A follow-up retracts the previous answer

Sending a follow-up while an answer is on screen SHALL sink the previous answer's
card back behind the bar over 300 ms, fading as it goes and leaving no empty card
behind. The request SHALL start when that retract is over; the new answer's
growth then opens the box again. With animations off the old text is
cleared at once and the request starts immediately.
Typing during the retract SHALL cancel the pending follow-up and leave the old answer
where it is.

#### Scenario: A second question

- **WHEN** the first answer is on screen and the user sends a follow-up
- **THEN** the card sinks back behind the bar, and the new answer rises out of it

#### Scenario: Typing during the retract

- **WHEN** the user types while the old answer is splitting away
- **THEN** the pending follow-up is dropped and the old answer stays until the text replaces it

### Requirement: The reply card rises from behind the bar

The reply card SHALL appear by rising from behind the bar: its top edge comes up
from the bar's top border, the part below that border is clipped, and its opacity
goes from transparent through half-transparent to fully visible as it rises. The card
SHALL NOT be drawn below the bar, and no second bar SHALL appear under the bar.

Leaving SHALL be the reverse: the card sinks back behind the bar and fades, with no
empty card left. Closing the panel, a follow-up's answer and its retract SHALL use
this movement. With animations off the card appears and goes in one frame.

#### Scenario: The first answer

- **WHEN** the first text arrives and the reply card appears
- **THEN** the card rises from behind the bar, turning opaque as it comes up

#### Scenario: The panel closes

- **WHEN** the panel closes with an answer on screen
- **THEN** the card sinks back behind the bar before the process exits
