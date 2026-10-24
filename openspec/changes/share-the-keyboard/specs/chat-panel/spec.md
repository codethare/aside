# Spec Delta

## MODIFIED Requirements

### Requirement: The panel is summoned, focused, and movable on both axes

The panel SHALL be summoned by a compositor keybinding that runs `aside`. It is a
layer surface, anchored to the bottom and left edges so that both margins are real
positions. It SHALL ask for **on-demand** keyboard interactivity: the compositor
focuses it when the pointer is over it (where the session follows the pointer) or
when it is clicked, and SHALL be able to focus another surface instead while the
panel is still mapped. It MUST NOT ask for exclusive interactivity, which makes
every view unfocusable for as long as the panel exists.

While the panel is focused it SHALL be fully typable, with the text input enabled;
when it loses focus the text input SHALL be disabled and any pre-edit dropped, and
when it is focused again the input SHALL be enabled again.

It SHALL be **movable by the user** in two ways that all move the same thing: the
left button pressed on the panel and moved, and `Alt+hjkl` with `Alt+Ctrl+hjkl` for
larger steps, in every mode. A drag SHALL follow the pointer from its first motion:
it MUST NOT wait for a hold, and a press released without moving SHALL move nothing.
Horizontal and vertical movement SHALL both take effect, the panel SHALL stay inside
the output on all four sides however it moves, and moving it MUST NOT resize it. The
position SHALL live only as long as the process: it is not written anywhere and is
not restored on the next run. The panel MUST NOT be tiled or decorated by the
compositor.

#### Scenario: Another window keeps the keyboard while the panel is up

- **WHEN** the panel is on screen and another window is focused, by a binding or by a click
- **THEN** keys reach that window, and the panel does not take them

#### Scenario: The panel is typable once it is focused

- **WHEN** the compositor focuses the panel, by the pointer coming to it or by a click
- **THEN** typing reaches the input line and the input method is enabled

#### Scenario: The panel goes quiet when it is not focused

- **WHEN** the panel loses focus in the middle of a composition
- **THEN** the pre-edit is dropped and the text input is disabled, and the keys go to
  whatever the compositor focused

#### Scenario: Esc is only the panel's while it is focused

- **WHEN** another window is focused and `Esc` is pressed
- **THEN** that window receives it and the panel stays up
