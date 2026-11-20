# Make the panel explain itself

## Why

The panel has the right states but not enough signs of them. A reader who scrolls up
to read an earlier part of a long answer is pulled back to the tail when it finishes;
new text can arrive below the view with nothing saying so. A paste that hits the
64 KiB cap is shortened, and the only report is a transcript note far from the line
that changed. A failed request puts the question back without saying it did. An
unfocused panel looks the same as a focused one except for a caret that is easy to
miss, so "why is typing dead" has no answer on screen. A long request says only that
it is running, not for how long, and the cancel key is documented rather than shown.
The font size needs a restart to change, and `--help` does not list the keys at all.

## What changes

- **Scroll intent.** Streaming appends and the end of a request clamp the view only
  when it is already following the tail. A reader who scrolled up keeps their place;
  a small `new content` pill sits at the replies' bottom-left until the view is back
  at the tail.
- **One toast.** Copy, a cut paste and a returned question all write one transient
  note in the bar. The paste note says the text was cut at the cap instead of
  silently shortening it; the returned note says the question is back in the line.
- **Return glow.** A failed request lights a faint accent rim on the bar for 700 ms,
  with the caret at the end of the restored question.
- **Focus rim.** An unfocused bar draws a very faint inner rim. It disappears when
  the compositor gives the panel the keyboard, by click or by a second summon.
- **Waiting hint.** While a request is in flight the bar's right side always reads
  `Esc cancel`; after three seconds it adds the elapsed whole seconds.
- **Typing interrupts.** The first typed character drops a leaving question and its
  particles at once, so the new line is the only thing in the bar.
- **Font flags.** `--font-size` and `--font-family` override the configuration for
  this run, which also makes `--preview` show particle density at another size.
- **Brighter particles.** The leaving particles use full white rather than the prompt
  ink, so a dense cloud on the dark glass still reads.
- **Keys in `--help`.** The one-page usage gains a compact key table.

## Capabilities

### Modified Capabilities

- `chat-panel`: scroll intent, clipboard and return feedback, focus and waiting cues,
  typing interruption, and the startup font flags.

## Impact

- `src/main.rs`: the toast, returned and waiting state, the follow rule, the flags.
- `src/panel.rs`: the new-content pill, focus/return rims, brighter particles.
- `src/font.rs`: no change; the flags feed the existing configuration.
- README, the man page and the live checklist.
