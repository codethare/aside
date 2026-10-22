# Spec Delta

## MODIFIED Requirements

### Requirement: The panel is a layer surface in three states

The panel SHALL be a `wlr-layer-shell` surface on the overlay layer, anchored to
the **bottom and left** edges — both, because a layer-shell margin only applies to
an anchored edge, and the panel is meant to be moved on both axes — horizontally
centred by its own left margin, with exclusive keyboard interactivity. Its bottom
edge SHALL sit 38 % of the output height above the screen's bottom edge. It SHALL
have exactly three states:

| State | Panel size (logical px, 1×) | Content |
|---|---|---|
| idle | 640 × 30 | the input bar alone |
| thinking | 60 × 60 | a filled circle with four blinking squares |
| answering | 640 × (bar + 12 + cap), fixed | the replies above, the bar below |

`cap` is the replies' height allowance, derived from the anchored gap and from
nothing else, so the answering panel is the same size for one line and for a
thousand. The replies inside it scroll. The surface carries the 20 px shadow
margin inside its own size.

#### Scenario: Summoned idle

- **WHEN** the panel opens with nothing asked
- **THEN** only the bar is on screen, and its bottom edge is 38 % of the output height above the screen's bottom edge

#### Scenario: Enter starts the ball

- **WHEN** the user presses `Enter` with a non-empty input
- **THEN** the request starts and the panel becomes a 60 × 60 square centred on the bar, painting a circle

#### Scenario: The first delta opens the box

- **WHEN** the answer's first text arrives
- **THEN** the panel takes its answering size once, the replies card rises out of the bar, and a second answer does not change the size

#### Scenario: A long answer pages inside a fixed box

- **WHEN** the answer is taller than the box
- **THEN** the panel keeps its size, the newest rows are the ones on screen, and scrolling reaches the older ones

### Requirement: The answer's height is capped by the bar's gap

The replies' allowance `cap` SHALL be the height of the gap between the bar and
the screen's bottom edge at the panel's anchored place — the same 38 % of the
output height — clamped by the room left above the bar and by `MAX_ROWS` rows, so
the panel never leaves the output. `cap` SHALL NOT depend on how much text the
transcript holds, and SHALL NOT change when the panel is moved. `MAX_ROWS` still
bounds what is drawn.

#### Scenario: One line and a thousand lines are the same size

- **WHEN** the transcripts hold one line and a thousand lines in turn
- **THEN** the answering panel is the same size both times, and only what is visible inside differs

#### Scenario: Moving the panel does not resize it

- **WHEN** the panel is dragged or stepped with `hjkl` while an answer is on screen
- **THEN** its size is unchanged, and only its position moved

#### Scenario: No output height yet

- **WHEN** the output's logical height is not known yet
- **THEN** the panel uses the cap of a 1080-pixel output and re-sizes once the real height is known

### Requirement: The panel is summoned, focused, and movable on both axes

The panel SHALL be summoned by a compositor keybinding that runs `aside`, and
SHALL take keyboard focus on appearing. It is a layer surface, anchored to the
bottom and left edges so that both margins are real positions. It SHALL be
**movable by the user** in three ways that all move the same thing: a press of the
left button held in place and then moved, and — in vim mode — `hjkl` with
`Ctrl+hjkl` for larger steps. Horizontal and vertical movement SHALL both take
effect, the panel SHALL stay inside the output on all four sides however it moves,
and moving it MUST NOT resize it. The position SHALL live only as long as the
process: it is not written anywhere and is not restored on the next run. The panel
MUST NOT be tiled or decorated by the compositor.

#### Scenario: Dragged sideways

- **WHEN** the user holds the left button and moves the pointer to the left
- **THEN** the panel moves left with it, and its size is unchanged

#### Scenario: Stepped sideways by the keyboard

- **WHEN** the user presses `l` three times in vim mode
- **THEN** the panel has moved 48 px right, or as far as the output's edge allows

#### Scenario: Nothing is written

- **WHEN** the panel has been moved and `aside` exits
- **THEN** nothing on disk records where it was, and the next run starts in its anchored place

### Requirement: Vim mode steps the window

`h`, `j`, `k` and `l` SHALL move the panel on the output, 16 logical px per press
in that direction, and `Ctrl+h`/`Ctrl+j`/`Ctrl+k`/`Ctrl+l` SHALL move it ten times
that. Both axes SHALL take effect. The move SHALL be applied through the same
clamped path as the drag, so the panel cannot leave the output on any side. A
press that cannot move — already against an edge — SHALL change nothing.

#### Scenario: Steps go where they say

- **WHEN** the user presses `l` three times
- **THEN** the panel's left edge is 48 px further right, or against the output's edge

#### Scenario: Ctrl takes a longer step

- **WHEN** the user presses `Ctrl+k`
- **THEN** the panel moves up 160 px, or as far as the space above it allows
