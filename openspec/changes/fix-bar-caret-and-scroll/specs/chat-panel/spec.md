## ADDED Requirements

### Requirement: The bar shows the line the input actually holds

The bar SHALL draw the input from the input's own caret: the caret is painted
between the characters before it and the characters after it, at the caret's byte
index, so `Left`, `Right`, `Home`, `End`, `Ctrl+Left` and `Ctrl+Right` are visible
and a backspace removes the character the caret is behind.

A line wider than the bar SHALL scroll so the caret stays inside the field, the
way a text field does. The line is drawn from a character boundary with the
partly scrolled glyph shifted rather than dropped, and the painter SHALL impose no
length limit of its own: a question of any length can be typed and corrected.

#### Scenario: The caret moves with the text

- **WHEN** a question has been typed and `Left` is pressed
- **THEN** the caret is painted one character earlier, and `Backspace` removes the character now before it

#### Scenario: A long question keeps its caret on screen

- **WHEN** the input is wider than the bar and more characters are typed
- **THEN** the line scrolls left under the caret, the caret stays inside the bar, and
  `Home` scrolls the beginning of the line back into view

#### Scenario: The composition stays with the caret

- **WHEN** an input method has a pre-edit at a caret in the middle of a long line
- **THEN** the pre-edit is drawn at the caret, underlined, and the text after it follows

### Requirement: The bar's line forms with the bar

While the panel is unfolding or folding, the bar's line SHALL be laid out at the
bar's settled geometry and painted only on the rows the card has reached, so the
text is revealed as the bar rises rather than being drawn above a card that does
not yet hold it.

#### Scenario: A line is not drawn before its bar

- **WHEN** the panel is part-way through its entrance
- **THEN** no part of the input line or its caret is painted outside the part of the card that has unfolded, and at the end of the entrance the line is exactly where it settles
