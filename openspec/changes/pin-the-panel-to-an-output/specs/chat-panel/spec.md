# Spec Delta

## ADDED Requirements

### Requirement: The panel sits on one output, known before it is mapped

`aside` SHALL create its layer surface on one output and finish every geometry
decision that depends on that output before the first frame: when `--output NAME` is
given, on the output of that name; otherwise on the only output the session has, when
it has exactly one. When the session has several outputs and no name is given, the
compositor chooses where the panel goes, and the panel adopts the size of the output
whose `wl_surface.enter` it receives — which the compositor sends after the first
buffer, so that one case MAY lay its first frame out at the assumed height and correct
itself on the next. That one frame is the only place an assumed height is allowed.

A name no output provides SHALL be one line on stderr naming the outputs that exist,
and no panel. The flag SHALL override no configuration key: an output name is a
property of the seat's session, not of the panel's settings, and it is not written
anywhere.

The bar's gap, the panel's width and the height cap SHALL be read from the logical
size of **the output the panel is on** — the one `--output` named, the only one there
is, or the one its surface entered — and MUST NOT be read from a fixed assumed height
while that output's size is known. A size reported by another output SHALL NOT
overwrite it, however late it arrives: a panel centred by a screen it is not on is the
bug this rule exists to prevent.

#### Scenario: A named output is honoured

- **WHEN** the session has `eDP-1` and `HDMI-A-1` and the panel is run with `--output HDMI-A-1`
- **THEN** the layer surface is created on `HDMI-A-1`, and its first frame is laid out at that output's logical size

#### Scenario: The only output is used without a flag

- **WHEN** the session has exactly one output and no `--output` is given
- **THEN** the panel is laid out at that output's logical size from its first frame

#### Scenario: An unknown output is refused, visibly

- **WHEN** `--output DVI-9` is given and no output has that name
- **THEN** nothing is mapped, and one line names the outputs the session does have

#### Scenario: Several outputs, no flag

- **WHEN** the session has two outputs and no `--output` is given
- **THEN** the compositor places the panel, and the panel adopts the size of the output it entered, from the frame after that output's `enter`

#### Scenario: Another output reports a size

- **WHEN** the panel is on a 2560×1440 output and a second, 1920×1080 output appears or changes mode
- **THEN** the panel's gap, width and height cap stay those of the 2560×1440 output it is on

#### Scenario: The panel is dragged to another output

- **WHEN** the compositor reports the surface entering another output
- **THEN** the panel takes that output's size, and its position is clamped to it
