# Spec Delta

## ADDED Requirements

### Requirement: The panel zooms as one piece

The panel SHALL have a zoom from 1.0 that scales the whole thing — glyphs, the row
height, the padding, the gap between the cards, the corner radius, the shadow margin
and the panel's own width — so a zoomed panel is a magnified copy of the same layout
rather than the same box with larger text. Zooming MUST NOT change the number of rows
the box holds.

Zoom SHALL be reachable three ways:

- `Ctrl+Alt+minus` SHALL zoom out and `Ctrl+Alt+equal` or `Ctrl+Alt+plus` SHALL zoom
  in, in every mode, including the prefix mode and copy mode;
- `Ctrl` held while the wheel turns over the panel SHALL zoom out on a notch towards
  the user and in on a notch away from it, and the wheel without `Ctrl` SHALL keep
  its present meaning;
- a two-finger pinch with the pointer on the panel SHALL zoom continuously with the
  gesture, and a cancelled gesture SHALL leave the zoom where it got to.

Zoom SHALL be clamped to a fixed range, and further clamped so the panel fits the
output it is on: zooming in on an output too small for the panel SHALL stop rather
than cut the panel off. Zoom SHALL live only as long as the process and MUST NOT be
written anywhere; a new run starts at 1.0 as far as zoom is concerned.

#### Scenario: The whole panel grows

- **WHEN** the zoom is raised one step while an answer is on screen
- **THEN** the text, the padding, the corner radius, the shadow and the panel's width all grow by the same factor, and the box still holds the same number of rows

#### Scenario: The keyboard works in every mode

- **WHEN** `Ctrl+Alt+plus` is pressed in typing mode, in the prefix mode or in copy mode
- **THEN** the panel zooms in, and nothing is typed into the input line and no copy-mode motion happens

#### Scenario: The wheel and the pinch

- **WHEN** the wheel turns with `Ctrl` held, or two fingers pinch, while the pointer is over the panel
- **THEN** the zoom follows, and the wheel without `Ctrl` still scrolls the replies

#### Scenario: The output is the limit

- **WHEN** the zoom is raised on an output where a larger panel would not fit
- **THEN** the panel stops growing at the size that fits, and nothing is drawn outside the output

#### Scenario: Nothing is kept

- **WHEN** the process exits and is started again
- **THEN** the panel is at 1.0 zoom, because the zoom is not stored
