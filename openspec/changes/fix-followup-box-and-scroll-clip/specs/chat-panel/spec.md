# Spec Delta

## MODIFIED Requirements

### Requirement: The panel is a layer surface in two sizes

The panel SHALL have exactly three states and two sizes, and the shape a state is
drawn as SHALL depend on the shape already on screen: a request in flight SHALL
keep the answering panel when it was asked from it, and otherwise be the bar.

| State | Panel size (logical px, 1×) | Content |
|---|---|---|
| idle | 640 × bar_h | the input bar alone |
| thinking, first question | 640 × bar_h | the same bar, with a light travelling its rim |
| thinking, follow-up | 640 × (bar_h + 16 + cap) | the same box, with the light on the bar's rim |
| answering | 640 × (bar_h + 16 + cap), fixed | the replies above, the bar below |

#### Scenario: A follow-up does not fold the panel shut

- **WHEN** the user sends a second question while an answer is on screen
- **THEN** the panel keeps its size and position, and the working light runs on its bar's rim

#### Scenario: The first question still grows the box

- **WHEN** the user sends the first question of a run
- **THEN** the panel is the bar with the light, and grows into the answering panel when the answer begins

### Requirement: The replies card shows whole rows

The replies card SHALL draw only rows that lie entirely inside it: a row that is
partly scrolled out above the card's top edge SHALL NOT be drawn, because its
descenders would be painted over the row below it.

#### Scenario: Scrolled by less than a row

- **WHEN** the view is scrolled so that the topmost row is half above the card
- **THEN** that row is not drawn, and the row below it sits where it always does

#### Scenario: Nothing overlaps

- **WHEN** any scroll offset is used
- **THEN** no two rows' ink shares a pixel
