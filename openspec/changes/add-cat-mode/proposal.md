# Give the panel a cat

## Why

The panel is two rounded cards and a bar. Everything it does well — the sandbox, the
ephemeral transcript, the keyboard table — is invisible until someone has already
adopted it, and nothing about it is a reason to try it. It looks like every other
floating prompt.

The cat is that reason: the panel becomes a character that peeks out of one of the
screen's edges, takes the question on its magnified paw, licks the paw while it thinks,
and answers out of its mouth. The default mode is untouched, so the change costs
nothing to a reader who does not want a cat.

## What changes

- One new configuration key, `mode`, `"default"` or `"cat"`. Absent means `"default"`.
  Anything else is one line naming the file and the line, and no panel — the same
  contract every other key has.
- Cat mode adds one more layer surface, the stage, which carries the cat and nothing
  else. The panel's own surface keeps its cards, its text, its keys, its scrolling, its
  selection and its markdown: the stage is painted beside them, never instead of them.
- The stage is placed by margins, appears at one of the four screen edges rolled once
  per run, hops to the panel's left side, and puts a paw on the input.
- The input bar is drawn as the magnified paw pad: same box, same height, same caret,
  same marquee, a different card.
- The reply card becomes the cat's bubble: the same box and the same behaviour, a tail
  that reaches the cat's mouth, growth out of the mouth instead of out of the bar, and
  the last answer stays on screen while the next request runs.
- While the request is open the cat licks the paw it just extended; when the answer
  starts the mouth opens and the bubble comes out of it; when the answer ends the paw
  is extended again and the bubble stays.
- With `animations = "false"` the cat is at its landed pose in the first frame, with no
  peek, no hop, no lick and no mouth.

## Cost

One more surface, one more shm buffer, and one more paint per frame while the cat is
moving — a square of 260 logical pixels, so a fraction of the panel's own frame. The
cat is drawn from the same primitives the chrome already uses: no asset, no dependency,
nothing written to disk, nothing to read at startup.

## Impact

- `src/cat.rs`: new — the edge roll, the stage's rect, the hop, the poses, the painter.
- `src/main.rs`: the stage surface's lifecycle, the mode's reading, the pose's clock,
  and the panel's shape while a request runs in cat mode.
- `src/panel.rs`: the pad and the bubble's tail and growth origin.
- `src/provider.rs`: the key is validated where its line number is known.
- `src/transcript.rs`: the last turn stays drawn while the next one has nothing to show.
- `README.md`, `docs/aside.1`, and the project's rule about the chrome.
- No dependency, nothing written to disk, no key binding changed.

## Non-goals

- A second skin. The key is shaped for one, but only `"default"` and `"cat"` land.
- The cat reacting to the pointer, to the text, or to anything but the request's phase.
- A cat in the default mode, or a second cat on a second output.
