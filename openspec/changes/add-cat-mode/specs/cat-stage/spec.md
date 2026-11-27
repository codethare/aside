# Spec Delta

## ADDED Requirements

### Requirement: The cat is a second surface, and it is not the panel's

In the cat mode `aside` SHALL create one further `wlr-layer-shell` surface, the stage,
on the panel's output, in the overlay layer, carrying the cat and nothing else. The
stage SHALL take no keyboard and no click: it SHALL be created with keyboard
interactivity `none` and an empty input region, and the panel SHALL keep both.

The stage SHALL be drawn at the panel's buffer scale and its scale objects, and SHALL
NOT change the panel's surface, size, input region or key table.

#### Scenario: A click on the cat

- **WHEN** the reader clicks where the cat is drawn in the cat mode
- **THEN** the click reaches what is behind the stage, and the panel's mode, selection and scroll do not change

#### Scenario: The stage never holds the keyboard

- **WHEN** the panel is summoned in the cat mode
- **THEN** the keyboard is the panel's, and typing works while the cat is moving

### Requirement: The stage lands on the input, and follows it

The stage SHALL be a square of `S` logical pixels, `S` being `260 * size` reduced to the
distance from the output's left edge to the panel card's left edge, and no stage SHALL be
created when that distance is under 180 logical pixels.

The stage SHALL be created once the panel's output is known, and SHALL be given a buffer
only after its own configure; until then the panel SHALL be drawn alone, and no frame of
the panel's SHALL wait for the stage.

The settled stage SHALL have its right edge on the input card's left edge and its
vertical centre on the input card's vertical centre, so that the cat's paw meets the
input card's left rim at that card's vertical centre, the reply card's bottom-left
corner lies inside the stage, and the stage overlaps no card.

Whenever the panel moves or changes size — a drag, a resize, a shape change — the stage
SHALL be placed again from the panel's new rect in the same frame.

#### Scenario: The paw is on the pad

- **WHEN** the cat has landed
- **THEN** the paw's tip is the input card's left rim at its vertical centre, and the two cards are untouched by the stage

#### Scenario: The panel is dragged

- **WHEN** the reader moves the panel with `Alt+hjkl` in the cat mode
- **THEN** the stage moves with it, and the paw is still on the pad

#### Scenario: A panel with no room to its left

- **WHEN** the panel's card leaves less than 180 logical pixels to its left
- **THEN** no stage is created, the panel is still the pad with the bubble, the bubble has no tail, and it grows from the bar

#### Scenario: The compositor places the panel

- **WHEN** the panel is summoned without `--output` on a compositor that places it
- **THEN** the panel is drawn and accepts typing before the stage exists, and the cat appears once the stage has been configured

### Requirement: The cat comes from one of the four edges

At startup in the cat mode the stage SHALL be placed at one of the four output edges,
rolled once from a seed of the clock and the process id, and SHALL be drawn cropped by
the window edge that sits on the output edge, so the cat is cut exactly by the screen.

From there the stage SHALL travel to its settled place over the entrance, and the
window SHALL travel with the cat, so that no frame draws the cat outside its own
surface.

#### Scenario: Every edge is reachable

- **WHEN** the roll is run over many seeds
- **THEN** each of the four edges is produced, and each places the stage flush with that edge

#### Scenario: The cat is never drawn outside its window

- **WHEN** the cat is hopping, at any point of the hop
- **THEN** everything drawn for it lies inside the stage's rect

### Requirement: The cat's poses follow the request

In the cat mode, with animation on, the stage SHALL draw, in order: a peek at the rolled
edge; a hop to the settled place; the paw extended onto the pad. Leaving `Idle` — sending
the question — SHALL end the entrance where it is, the stage SHALL be at its settled
place from that frame, and the request's pose SHALL take over. While nothing has come
back, from 300 ms after the question, the cat SHALL lick the extended paw, and the lick
SHALL stop at the first text. When the answer's first text arrives the cat SHALL open and
close its mouth once. When the answer ends the cat SHALL extend its paw again, and the
bubble SHALL stay.

With `animations = "false"` the stage SHALL be at its settled place in the waiting pose
in the first frame, with no peek, no hop, no lick and no mouth.

#### Scenario: A request that ends before the lick

- **WHEN** an answer's first text arrives within 300 ms of the question
- **THEN** the cat never licks, and its mouth opens once for the bubble

#### Scenario: Sending before the cat has landed

- **WHEN** the question is sent while the cat is still peeking or hopping
- **THEN** the cat is at its settled place in that frame, with its paw on the pad, and no pose of the entrance is drawn after it

#### Scenario: The lick stops at the first text

- **WHEN** an answer streams for several seconds
- **THEN** the cat licks until the first text and answers for the rest, and it is not licking while the answer streams

#### Scenario: The paw comes back after the answer

- **WHEN** an answer finishes in the cat mode
- **THEN** the cat extends its paw again and the bubble is still on screen

#### Scenario: Animations off

- **WHEN** `animations = "false"` and `mode = "cat"`
- **THEN** the first frame already has the cat beside the pad with its paw out, and no pose follows
