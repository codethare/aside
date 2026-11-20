# Proposal

## Why

When a request ends, the question in the bar leaves as one accelerated, fading line.
It reads as the line being scrolled out of the field, not as the question being
finished with: the whole word shape stays intact until it is off the edge, and the
movement is the same for a question that was scrolling and one that never moved.

The wanted ending is a removal, in two beats: the visible line first comes apart into
small particles, then those particles are drawn out of the bar in the direction the
marquee was travelling at the instant the request ended. A long question that was
running left is pulled out to the left; one that was coming back is pulled out to the
right; a short question, which never moved, decomposes and leaves to the left.

## What changes

- **The leaving timeline is split in two.** `DECOMPOSE` (120 ms): the glyphs fade
  out while the particles that replace them fade in at the same positions, with a
  small seeded jitter — the line comes apart. `EXTRACT` (220 ms): the particles
  accelerate toward the chosen edge, staggered by their seed, and fade to nothing.
  `SUCK` becomes `DECOMPOSE + EXTRACT` (340 ms), and the question is cleared at that
  end as it is now.
- **The instant is frozen.** The marquee's offset and its direction are read once, at
  the moment `leaving` is set (`age = elapsed - since`), and every later frame uses
  those values. The current code re-reads the marquee while leaving, so a line that
  turns round during the exit can change edge mid-flight; after this change it cannot.
- **Particles come from the visible line only.** For each visible glyph the bar emits
  a fixed number of particles (8), seeded from the glyph's byte index and the particle
  index, with a hard cap of 480 particles. Their start is the glyph's advance box at
  the frozen scroll; their exit is the frozen direction's edge, through the same
  travel/limit arithmetic `leaving_offset` uses today. Particles are one logical-pixel
  squares in the prompt ink and are clipped to the bar's row and width.
- **Nothing else moves.** Layout, the bar's size, the caret and the input line are
  unchanged; typing still drops the leaving question at once; the transcript still
  draws no question.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-panel`: the question now decomposes into particles before being extracted;
  the exit direction and start offset are frozen at the instant the request ends.

## Impact

- `src/panel.rs`: a pure leaving-timeline helper, deterministic particle seeding, and
  the two drawing stages in `draw_bar`'s pending path.
- `src/main.rs`: `SUCK` gains the decompose phase; nothing else in the state machine.
- No dependency, no file, no layout or frame-cost change beyond the visible bar row.
