## MODIFIED Requirements

### Requirement: The panel is correct at 1x, 1.5x and 2x

The panel SHALL honour `wl_output.scale`, INCLUDING the case where the display's
scale changes while it is open, and the first commit of a surface MUST match the
size the compositor configured. Subpixel text filtering SHALL be used only at
scale 1, where it is legible, and grayscale antialiasing at higher scales.

When the compositor offers `wp_fractional_scale_v1` and `wp_viewporter`, the panel
SHALL prefer the fractional scale over `wl_output.scale`, ignore later integer
scale changes while that preference stands, and honour a new preferred scale while
it is open. It SHALL draw at `ceil(preferred / 120)` buffer pixels per logical
pixel, leave `wl_surface.buffer_scale` at 1 as that protocol asks, and set the
viewport destination to the surface's logical size, so text keeps its logical size
and is not upscaled from a smaller buffer. Without the protocols the integer scale
is used, with no viewport.

#### Scenario: Fractional and integer scaling

- **WHEN** the panel is mapped on a 1x, 1.5x and 2x output in turn
- **THEN** text is the same logical size on all three and no colour fringing appears above scale 1

#### Scenario: A 1.5x output

- **WHEN** the compositor prefers a scale of 180/120 on the panel's surface
- **THEN** the buffer is drawn at 2x, the buffer scale is 1, the viewport destination is the logical size, and the text is physically 1.5x with grayscale antialiasing

#### Scenario: The preferred scale changes while open

- **WHEN** the surface moves to an output with a different preferred scale
- **THEN** the font, the rows and the buffer are rebuilt once and the panel re-wraps at the new width, as an integer scale change does

#### Scenario: No fractional protocol

- **WHEN** the compositor offers neither `wp_fractional_scale_v1` nor `wp_viewporter`
- **THEN** the panel uses the integer `wl_output.scale`, sets no viewport destination, and starts as before

#### Scenario: A protocol error never becomes a spin

- **WHEN** the compositor reports a protocol error, including a first-commit size mismatch
- **THEN** `aside` reports it and exits, and no failure path leaves the process burning CPU

#### Scenario: No buffer before the first configure

- **WHEN** the compositor has not yet sent a configure
- **THEN** `aside` commits no buffer and paints nothing, because a strict compositor ends the connection

#### Scenario: The compositor never configures

- **WHEN** no configure arrives within the deadline
- **THEN** `aside` reports that and exits non-zero, rather than leaving a process that is running and invisible

#### Scenario: The compositor closes the connection

- **WHEN** the compositor goes away while a panel is on screen, so the socket reports a broken pipe
- **THEN** `aside` exits quietly with status 0, because the session ending is not a failure of the panel
- **AND** every other dispatch failure, and any protocol error, is still reported and exits non-zero
