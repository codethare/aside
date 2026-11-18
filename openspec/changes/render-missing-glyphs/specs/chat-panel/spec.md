## ADDED Requirements

### Requirement: Every character is drawn or marked

The renderer SHALL NOT silently drop a character. A character no face covers SHALL
advance by the space's width and SHALL be drawn as a hollow box; measurement and
drawing SHALL use the same advance. A tab SHALL advance four space widths and draw
no ink, keeping the stored tab. On the first uncovered character the panel SHALL
append the system's script families and emoji faces to the chain, once and
deduplicated, and that scan SHALL NOT run while the font loads.

#### Scenario: A character no face covers

- **WHEN** an answer contains a character no face in the chain has a glyph for
- **THEN** a hollow box is drawn in its place, the text after it starts one space further, and a pointer over that space names the character it stands for

#### Scenario: A tab inside a code block

- **WHEN** a fenced block or a pasted question contains a tab
- **THEN** the following text starts four space widths further and no box is drawn for the tab

#### Scenario: A script outside the default chain

- **WHEN** the answer first contains a Thai or Devanagari character and the system has a Noto face for it
- **THEN** that face is added to the chain on that miss and the character has ink, while a Latin answer never reads it or scans the font database

#### Scenario: A system with no face for the script

- **WHEN** no installed face covers the character
- **THEN** the box is drawn, and the fallback scan is not repeated for the next one
