# Make the cat's stance and motion less wooden

## Why

The cat now has a Notion palette, but its stance is still a flat, axis-aligned
loaf: the head looks straight ahead, the ears sit square, the tail is a fixed
curl, and every pose is a single eased curve on a fixed clock. The entrance, the
lick and the paw reach read as a diagram being played, not a character moving.

## What changes

- **A modern stance.** The head sits higher and further forward (0.65 w, 0.34 h),
  and rests with a 6 degree tilt toward the panel. The ears turn with the head;
  the face turns around the mouth, so the mouth stays the fixed point the bubble's
  tail and growth use. The body is taller and asymmetric, and the tail carries a
  small loop.
- **Spring, not linear.** The paw reaches on an ease-out-back curve that
  overshoots by at most 3 % and settles exactly on `(S, S/2)`. The stage's hop
  moves horizontally on a smoothstep rather than a linear lerp.
- **Irregular life.** The lick is a 620 ms flick with a pause and a head bob, not
  a metronome sine. The speaking mouth is a two-part envelope, not one bump. The
  tail sways and an ear twitches during the active poses.
- **A blink.** While the cat waits, the eyes close for 260 ms every 5 s. The stage
  already wakes on a 250 ms idle tick, so the blink costs two frames per cycle,
  not a continuous animation.

## Impact

- Code: `src/cat.rs` (the stance, the Act model, the painters, `hop`) and
  `src/main.rs` (`stage_wake` for the waiting blink).
- Spec: a `cat-stage` delta for the stance and the motion.
- Docs: README, man page and `CLAUDE.md` rule 4 describe the tilted stance and the
  spring-eased motion.
- No new key, surface, dependency, asset or file; the paw contact, the mouth
  point, the landing and every pose deadline stay.
