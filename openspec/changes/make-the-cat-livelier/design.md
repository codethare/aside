# Design: a modern, spring-eased cat

## Intent

Keep every geometric contract the cat mode already has — the landing, the paw
contact at `(S, S/2)`, the mouth on the outline, the bubble's growth origin, the
edge roll and the pose deadlines — and make the character look and move as if it
were drawn and animated today: an angled head, unequal curves, overlapping
motion, overshoot that settles, and a blink.

## The stance

| Part | Now | New |
| --- | --- | --- |
| Head | `(0.62w, 0.38h)`, r `0.16w` | `(0.65w, 0.34h)`, r `0.16w` |
| Rest tilt | none | `+0.10 rad` (about 6 deg, toward the panel) |
| Body | symmetric loaf | taller, asymmetric chest and rump |
| Tail | a C curl | a C with a small loop at the tip |
| Face | front-on | turned with the head around the mouth |
| Ears | square to the head | turned with the head, one twitch |

**Two pivots, and both are load-bearing.** The ears rotate about the head's
centre, so their bases stay on the head ring. The eyes, nose, smile and whiskers
rotate about the mouth point, so the mouth stays exactly where `mouth_at` says it
is. The bubble's tail and its growth origin therefore never move under a tilt,
and the smile is still joined to the mouth. The head ring is a circle, so it is
invariant under both rotations.

The body's neck points stay on the ring and do not rotate; the head's tilt is a
face-and-ears turn, not a whole-head orbit. At 6 degrees the difference is
invisible; the contract is not.

## The motion model

`Act` gains `tilt`, `tail_sway`, `ear`, `tongue_sway` and `blink`. Each pose
computes them from its own clock, so one pose can overlap the next without a
second clock:

| Pose | Motion |
| --- | --- |
| Peek | the head rises on the existing ease, with the tilt arriving over the rise |
| Hop | the stage's horizontal travel is a smoothstep; the cat tilts, the tail is up and sways |
| Arm | `ease_out_back` with c1 0.9: at most 3 % past the contact, back to exactly 1 |
| Waiting | rest tilt, a fixed curl, a blink |
| Lick | 620 ms cycle: a 50 % flick, then a pause; head bob, tongue sway, tail sway, ear twitch |
| Speak | two bumps: the main `sin^0.7` and a second, smaller one after 55 %; a head nod |
| Extend | the old press, plus a small third harmonic and a spring settle |

`tail_sway` and `ear` use different, incommensurate frequencies, so the cat never
looks like it is ticking. All of it is a pure function of `Duration::since`, so a
preview frame and a live frame agree.

## The blink

`cat::blink(since)` returns 1 for 260 ms starting 1.2 s into each 5 s of waiting,
so the first blink comes after the cat has settled. The stage already has a 250 ms
idle tick (`IDLE_WAKE`); `stage_wake` returns the blink interval while `Waiting`,
so the loop samples the 260 ms window at least once and the eyes close for one
frame, then open. This is two frames per 5 s, not an idle animation loop. The
closed eye is a short horizontal stroke where the dot was.

## Risks

- **The overshoot at the pad.** Reach past 1 pushes the mitten into the card's
  rim and clips it at the stage edge. The curve is capped at 3 % and returns to
  exactly 1; the landed frame is the identity.
- **The tail loop can cross the body.** The loop's control points stay left of
  the body's back; the preview is the check.
- **The 250 ms blink sampling.** A 260 ms window guarantees one sample, but a
  compositor that delivers frames late may show a longer or shorter blink. It is
  a blink, not a state.
- **The mouth stays fixed while the face turns.** At 6 degrees this reads as a
  head cock, not a detached face. Larger tilts would need the bubble's origin to
  follow, which this change deliberately avoids.
- **The surface path is unchanged and still unverified without a compositor.**
