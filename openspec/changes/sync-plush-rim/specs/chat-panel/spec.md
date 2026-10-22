# Spec Delta

## MODIFIED Requirements

### Requirement: The frame matches wlmenu's frame

The two cards SHALL be drawn as `plush` draws its panel: a premultiplied
translucent body, a **26 px** shadow margin holding both drop-shadow layers —
`(blur 9, alpha 120, offset 2)` and `(blur 20, alpha 95, offset 6)` — measured from
each card's rounded outline so the falloff wraps the corners, antialiased corners,
and a rim made of three strokes: a dark underside at the bottom edge, a lit rim at
`#ffffff5a` whose profile holds through the top corner arcs and dies by four fifths
of the card's height, and a dark band one pixel inside the rim that is absent at
the top edge and whole at the bottom. The bar and the replies SHALL share one body
tone, `BG_PROMPT`, and the **16 px** between them SHALL show nothing but their own
shadows — the gap clears the wider layer's 14 px reach above a card. The thinking
ball SHALL use the same body, rim and shadow.

#### Scenario: One panel, two products

- **WHEN** a screenshot of `aside`'s answering panel and one of `plush`'s are laid side by side
- **THEN** their shadow width and falloff, rim brightness, inner band and corner shape agree

#### Scenario: The gap clears the shadow

- **WHEN** the two cards are laid out
- **THEN** the space between them is 16 px, and no card's shadow reaches the other card's body

#### Scenario: One grey

- **WHEN** the answering panel is inspected row by row
- **THEN** the bar card and the replies card are the same tone, and no reply block is filled differently from the card it sits in
