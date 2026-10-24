# Spec Delta

## MODIFIED Requirements

### Requirement: The panel is summoned, focused, and movable on both axes

The panel SHALL be summoned by a compositor keybinding that runs `aside`, and
SHALL have the keyboard when it appears, without a click. It is a layer surface on
the overlay layer, asking for **exclusive** keyboard interactivity, which is the
only mode in which the compositor gives a layer surface the keyboard on its own.
While the panel is up the keyboard SHALL stay with it; `Esc` closes the panel and
gives it up. It SHALL be anchored to the bottom and left edges so that both margins
are real positions. It SHALL be **movable by the user** in three ways that all move
the same thing: a press of the left button held in place and then moved, and — in
vim mode — `hjkl` with `Ctrl+hjkl` for larger steps. Horizontal and vertical
movement SHALL both take effect, the panel SHALL stay inside the output on all four
sides however it moves, and moving it MUST NOT resize it. The position SHALL live
only as long as the process: it is not written anywhere and is not restored on the
next run. The panel MUST NOT be tiled or decorated by the compositor.

#### Scenario: Typed into without a click

- **WHEN** the compositor runs `aside` from a keybinding and the panel appears
- **THEN** the input line has the keyboard, and typing reaches it without the pointer

#### Scenario: The keyboard does not wander off on its own

- **WHEN** the panel is up and another window is clicked
- **THEN** the panel keeps the keyboard, and `Esc` closes it and hands the keyboard back

#### Scenario: Dragged sideways

- **WHEN** the user holds the left button and moves the pointer to the left
- **THEN** the panel moves left with it, and its size is unchanged

#### Scenario: Stepped sideways by the keyboard

- **WHEN** the user presses `l` three times in vim mode
- **THEN** the panel has moved 48 px right, or as far as the output's edge allows

#### Scenario: Nothing is written

- **WHEN** the panel has been moved and `aside` exits
- **THEN** nothing on disk records where it was, and the next run starts in its anchored place
