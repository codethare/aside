# Spec Delta

## ADDED Requirements

### Requirement: The panel's own motion is a setting

`aside` SHALL read `animations` from its configuration file, accepting `"true"` and
`"false"` and refusing anything else with the file and the line that carried it. With
the key absent, the panel is animated.

With `animations = "false"` the panel SHALL change state without the transitions: its
opening, its growth into the replies and its closing SHALL each take one frame, and no
intermediate frame SHALL be drawn for them. What the panel uses to report that
something is happening — the light running round the bar's rim while a request is open,
the scroll indicator showing and fading while the view moves — SHALL stay: they are
cues about the request and the view, not the panel moving.

The setting SHALL NOT change the panel's sizes, the fixed reply box, or any key.

#### Scenario: Animations off opens and closes in one frame

- **WHEN** `animations = "false"` and the user presses the summoning key
- **THEN** the panel appears at its final size with no spreading line and no unfolding card, and closes in a single frame when it goes

#### Scenario: Animations off keeps the working light

- **WHEN** `animations = "false"` and a request takes longer than half a second
- **THEN** the light still runs round the bar's rim until the first text arrives

#### Scenario: A value that is not a boolean is refused

- **WHEN** the configuration file contains `animations = "sometimes"`
- **THEN** one line names the file, the line and the two accepted values, and no panel opens

#### Scenario: The key is absent

- **WHEN** the configuration file has no `animations` key
- **THEN** the panel is animated, exactly as before this change
