# Spec Delta

## MODIFIED Requirements

### Requirement: The panel is summoned, focused, and movable on both axes

The panel SHALL be summoned by a compositor keybinding that runs `aside`. It is a
layer surface, anchored to the bottom and left edges so that both margins are real
positions.

It SHALL take the keyboard by itself when it appears: it maps asking for **exclusive**
interactivity, and SHALL drop to **on-demand** once the compositor has configured the
surface, no later than 100 ms after that, and immediately when the panel starts
closing. While it asks for on-demand — which is every moment after that prime — the
compositor focuses it when the pointer is over it or when it is clicked, and SHALL be
able to focus another surface instead while the panel is still mapped. The exclusive
prime MUST be bounded: an exclusive mode kept for as long as the surface exists makes
every view unfocusable, and that is not what this panel asks for.

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

#### Scenario: The panel is typable as soon as it appears

- **WHEN** the user presses the summoning key on a compositor that does not follow the
  pointer
- **THEN** the panel maps with the keyboard, and the next keystroke reaches the input
  line without the pointer being moved onto it or a click

#### Scenario: Another window keeps the keyboard while the panel is up

- **WHEN** the panel is on screen and another window is focused, by a binding or by a click
- **THEN** keys reach that window, and the panel does not take them

#### Scenario: The panel goes quiet when it is not focused

- **WHEN** the panel loses focus in the middle of a composition
- **THEN** the pre-edit is dropped and the text input is disabled, and the keys go to
  whatever the compositor focused

#### Scenario: Esc is only the panel's while it is focused

- **WHEN** another window is focused and `Esc` is pressed
- **THEN** that window receives it and the panel stays up

### Requirement: A second summon puts the running panel away

Summoning `aside` while an instance is already running SHALL close that instance — the
same closing the `Esc` key starts — and the new launch SHALL exit zero. It MUST NOT
start a second panel and MUST NOT raise the running one: a layer surface on the
overlay layer is never behind anything, so the second press means "not now". A second
summon that arrives while the panel is already closing SHALL change nothing.

#### Scenario: Second summon closes instead of duplicating

- **WHEN** `aside` is running and the user presses the summoning key again
- **THEN** the running panel rolls up and exits, and no second panel appears

#### Scenario: Second summon while the request is out

- **WHEN** the user summons the panel again before the answer has finished
- **THEN** the panel closes, and the request dies with the process that owned it

#### Scenario: Second summon while the panel is already closing

- **WHEN** the user summons the panel again while the closing animation is running
- **THEN** the closing continues uninterrupted and the new launch exits zero

## RENAMED Requirements

- FROM: `### Requirement: A second launch raises the running panel`
- TO: `### Requirement: A second summon puts the running panel away`
