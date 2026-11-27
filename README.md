# aside

A keyboard-summoned AI chat panel for Wayland. Press a key, type a question, watch
the answer stream in, ask a follow-up, take a line out to the clipboard. The
transcript lives in this process's memory and dies with it: no history, no cache,
no log file.

Behaviour is specified by the deltas under `openspec/changes/`; `openspec list`
shows what is open and `openspec show <change>` reads one. Deltas in different
changes compose in the order they were written; `README` only says how to run the
program and why it is built the way it is.

## Run

```sh
aside
```

It needs a Wayland session, and the programs on `PATH`: `bwrap`, `curl`,
`wl-copy` and `wl-paste` (both from `wl-clipboard`), and a compositor that speaks
`wlr-layer-shell`. Bind it in your compositor,
for example in Hyprland:

```
bind = SUPER, A, exec, aside
```

Press the key again while it is open and the panel does what the keyboard says: with
the keyboard it puts itself away — the same closing `Esc` starts — and without it, it
takes the keyboard back with the transcript untouched. `river` hands a layer surface
the keyboard only when the surface maps, so that second press is an unmap and a map of
the same panel, and the panel blinks for one frame.

That remap is a request, not a promise: `river` applies the window manager's own focus
request over a layer surface's, and `tailrace` sends one whenever its cached focus has
moved on — so a take can come back with no keyboard at all. The panel never relies on
it. The caret is drawn only while the panel actually holds the keyboard, so a panel
that will not take your typing looks like one; and a take whose keyboard does not stick
— one taken away again inside a fraction of a second counts as none — turns the next
press into a close, so the key that summoned the panel can always dismiss it.

The panel draws its own card and its own animation, so Hyprland should not animate
the surface a second time:

```
layerrule = no_anim, aside
```

Configuration is optional: `~/.config/aside/config.toml` with flat `key = "value"`
lines, and eight keys — `model`, `base_url`, `proxy`, `font_size`, `font_family`,
`animations`, `mode` and `instructions`:

```toml
model = "deepseek/deepseek-v4.1-flash"
font_size = "24"                 # 8 to 72 logical px; the bar follows it
font_family = "Noto Sans Mono"    # a name nothing provides falls back to the chain
animations = "true"               # "false" skips the opening, the growth, the closing and the particle exit
mode = "default"                 # "cat" is the same panel with a cat in it
instructions = "Answer in Chinese."          # the reader's own constraints: one
instructions = "At most three sentences."    # rule a line, joined in this order
```

`mode = "cat"` is the same panel staged around a black cat. It comes out of one of
the screen's four edges, hops to the panel's left side and takes the input on its
magnified paw, which is the bar drawn as a paw pad. While the request runs it licks
that paw, and the answer arrives in a bubble that comes out of its mouth and stays
there, with the last answer still in it, until the next answer starts. Every key,
the scrolling, the selection, the markdown and the clipboard are the same in both
modes; `"default"` is the panel described above, unchanged.

The configuration is read once at startup: a changed size needs a new run, or
`--font-size`/`--font-family` on the command line.

`instructions` is what an `AGENTS.md` is for a coding agent: every request leads with
them as a `system` message, so a question and its follow-ups all answer under the same
rules, and the panel never draws them. The key may repeat — the lines join in file
order with a newline — and it is the only key that may. An empty line, or more than
2000 characters in total, is refused with the file and the line rather than cut.

A size that is not a size is an error naming the file and the line, and nothing
opens. The credential is environment-only:

```sh
export OPENCODE_API_KEY=...
```

Flags override both: `--model`, `--base-url`, `--proxy`. `--output NAME` pins the
panel to one output by name; without it the compositor places it, which on
Hyprland is the focused monitor — with several outputs and no name, a click on
another output does not close the panel. `--font-size PX` and `--font-family NAME`
override the configuration for the run, which is also how `--preview` shows another
size. `aside --preview out.ppm` renders a
scripted transcript to an image without opening a window, plus `out.ppm.decompose.ppm`
and `out.ppm.extract.ppm` for the question's leaving stages; this is how the layout
and the animation are checked on a machine with no compositor.

## What it looks like

A `wlr-layer-shell` surface on the overlay layer, asking for **on-demand** keyboard
interactivity, once: a compositor gives the keyboard to an interactive layer surface
when it maps, so the panel is typable the moment it appears, and it stays the
compositor's to move afterwards, so the panel never blocks typing elsewhere. The
request is never changed on a mapped surface — Hyprland answers such a change by
re-running its mouse focus and handing the keyboard back to the window under the
pointer. Anchoring is what keeps it out of the way;
`Esc` closes it. Anchored to the bottom of the output and centred, its bottom edge
38 % of the output height above the screen's bottom edge: the field lands just below the vertical centre, where the eye already
is, and the replies grow upwards out of it. The bottom anchor is what keeps the
bar in one place on screen; a top anchor would carry it downwards by a row per
wrapped line. The chrome is `plush`'s: a 26 px drop shadow measured from each
card's outline, a premultiplied translucent body, a rim made of a lit top edge
fading out by four fifths of the card, a dark underside at the bottom and a dark
band one pixel inside it, and antialiased corners. It has three states:

| State | Panel size | What is drawn |
|---|---|---|
| idle | 640 × 30 | the bar alone |
| thinking | 640 × 30 | the same bar, with a light running round its rim |
| answering | 640 × (replies + 16 + bar) | the replies above, the bar below |

Two cards of one glass, 16 px apart: the replies in the upper one, the field in
the lower one, and nothing but their shadows between them. Only the model's text
is on screen: an answer, a failure and the panel's own notes, separated by 8 px
gaps. A question you sent is kept for the next request and is not part of the
transcript: the bar keeps it in view while the request runs, and a question wider
than the bar scrolls slowly out to its last character, holds, comes back, and turns
round again for as long as the request is unfinished. The bar is the question's
while it is out — no caret on a line nobody can type into. When the request ends it
comes apart into particles where it stands, and those are pulled out of the bar:
the scroll and the heading are frozen at that instant, so a line running out leaves
to the left, one coming back leaves to the right, and a short question leaves to the
left. The particles accelerate and fade, so the bar is an empty, typable line again
with the line leaving rather than blinking out. A key pressed in that moment drops
it at once. If the request fails instead of ending, the question goes back into the
input line, caret at its end, ready for `Enter` — unless you have started typing
something else, in which case your text wins; the bar says the question returned and
glows for a moment. While an answer streams, scrolling up keeps your place: new text
below the view raises a small `new content` mark at the bottom of the replies, and
scrolling back to the tail clears it and resumes following. An unfocused bar carries
a faint inner rim, so a keyboard that has gone elsewhere is visible; while a request
runs the bar's right side reads `Esc cancel`, adding the elapsed seconds after three.
The request carries at most 512 KiB
of turn text; older turns are dropped from it and their text is freed, and the
transcript says when that happened.
`Up`/`Down` mark the highlighted line with brighter ink two pixels in from its
neighbours, so there is no mark in the margin at all. The replies arrive with the
panel's own growth and are not animated a second time; a follow-up first draws the
previous answer's card up into its top border, then the new answer drips down from
that border. The wheel scrolls the replies while the pointer is over them — five rows a notch, a touchpad's own
delta otherwise — and does nothing over the bar. The notch is counted from whichever step field the
compositor fills in, so a modern wheel and an old one move the same distance, and a
touchpad's fraction is carried rather than rounded away. A burst of quick notches
travels farther as it runs: each step is half again as far as the one before, up to
four times five rows, and 150 ms without a notch returns it to five rows. A touchpad
counts one gesture as one step, so a single slide moves exactly with the fingers and
it is repeated quick flicks that grow.

The sizes are the panel's own; the layer surface adds the shadow margin on every
side and sits that margin inside the visible gap. Only the two cards take pointer
input: the shadow margin and the 16 px between them are click-through, so a window
seen there can be clicked and typed into while the panel is up. On a compositor offering
`wp_fractional_scale_v1` and `wp_viewporter`, the panel draws at the next integer
scale and lets the viewport shrink that buffer to the preferred size, so a 1.5×
output is not an upscaled 1×. The answering panel is a
**fixed** size: the bar, the 16 px between the cards, and a box as tall as the
bar's own gap above the screen's bottom edge (38 % of the output, clamped by the
room left above it). It does not grow as the answer does — a longer answer scrolls
inside the box, and the 3 px scroll indicator on the right edge shows while the view
is moving and then fades out of the way. It can be dragged: a press anywhere in the
card's right 16 px padding column takes hold of it — where it was pressed if that
was on the thumb, the thumb's middle if it was on the track — and moving carries the
view with it, from the transcript's first line to its last. Held, it stays at full
ink, however long the button is down. While a burst accelerates, that strip carries
the multiplier in small type on its own column — `1.5x`, `2x`, up to `4x` — set
smaller than the answer's text, with the strip cut around the number rather than drawn
through it, and the number goes 400 ms after the last notch while the strip fades on
its own clock. Holding the
left button on the bar and moving drags the panel, in both axes, from the first
motion; it stays on the output, keeps its size, and where it was is never written
down. Dragging across the replies selects whole lines instead — press on a line,
drag, release — and `Alt+C` or `Ctrl+C` copies every selected line, one per line.
`Esc` clears a selection before it closes the panel.
State changes are animated, and always as one card changing shape, painted inside
one buffer, so the surface is resized once per state and never per frame. Opening
takes 520 ms: a line spreads out of a point at the centre, and once it has the
panel's width the reply card drips down from its top border. Sending a question moves nothing: the
bar stays, and — once the request has taken half a second, which a fast answer never
reaches — a light runs round its rim, one lap in 1.4 s, until the first text
arrives, which grows the panel out of the bar in 460 ms — that growth is the reply's
entrance: the reply card hangs from its top border and drips downward, opaque from
the first visible strip, with a small spring that overshoots and settles; the top
never detaches. A follow-up draws the previous answer's card back up into that top
border in 300 ms, accelerating as it goes and leaving no empty card behind; the
light runs on the bar's rim until the new answer drips down. Closing is the same
suction into the top, and then the process exits.

## Keys

| Key | Mode | Action |
|---|---|---|
| `Enter` | typing | send |
| `Esc` | typing | cancel a streaming answer, or close the panel |
| `Esc` | prefix | leave the mode, cancelling nothing |
| `Esc` | copy | clear the selection; leave copy mode when there is none |
| `Ctrl+T` | typing | enter the prefix mode; the next key is a settings command |
| `[` | prefix | enter copy mode |
| `+` / `=` | prefix | two pixels larger, up to 72 |
| `-` | prefix | two pixels smaller, down to 8 |
| `Alt+hjkl` | any | move the **panel** 16 px left / down / up / right |
| `Alt+Ctrl+hjkl` | any | the same, 160 px |
| `Ctrl+Alt+-` / `Ctrl+Alt++` | any | make the **panel** smaller / bigger, 15 % a press |
| `Ctrl+Alt+BackSpace` | any | back to the panel's default size |
| `Ctrl` + wheel | any over the panel | resize the panel, 5 % a notch |
| two-finger pinch | any over the panel | resize the panel, following the fingers |
| `Up` / `Down` | typing | move the highlighted transcript line |
| `PageUp` / `PageDown` | typing | move it a page |
| `Alt+C` / `Ctrl+C` | typing, copy | copy the selection, or the highlighted line |
| `Ctrl+V` | typing | paste the clipboard into the input line |

The question is capped at 64 KiB: a longer selection is pasted up to the cap, on a
whole character, and the bar says it was cut.

Two sizes, deliberately separate. `Ctrl+T` then `+`/`-` changes the **text** by two
points. `Ctrl+Alt` `+`/`-` — or `Ctrl` with the wheel, or a pinch — changes the
**panel's box**: the width and the room the replies get, between 75 % and 200 % of the
design size. Both sides stop together, at the point where either of them would leave
the output, so the box never ends up wide and flat because the height ran out first. The text keeps its size,
so a bigger box shows more of it and the lines wrap at the new width. Neither is
written anywhere.

Copy mode is tmux's, cut down to what the panel has to walk. `h`/`l` move a
character, `0`/`$` to the ends of the line, `j`/`k` a line, `w`/`b` a word,
`Ctrl+d`/`Ctrl+u` half a page, `Ctrl+f`/`Ctrl+b` (or `PageDown`/`PageUp`) a page,
`g`/`G` the first and last line. `Space` starts a selection and `Enter` or `y`
copies it — or the line the cursor is on — and leaves; `q` leaves, `Esc` clears a
selection first. After a copy the bar says `copied N lines` for a moment. A table
row is copied as TSV: the screen's padding is removed and the cells are separated by
tabs, so pasting into a spreadsheet fills its columns.

`Ctrl+T` is tmux's prefix shape rather than a mode toggle: it takes exactly one
key. `[` enters copy mode and `+` (`=` unshifted) and `-` change the text size two
pixels at a time inside the range the configuration accepts — the face is read
again, the transcript is laid out again in it and the surface resized once, and the
size is not written down. Everything else leaves the prefix and does nothing, and
later settings commands belong there. While it waits, the bar takes no input,
changes tone and reads `press [ for copy mode, + / - for text size`, so the next key
is never swallowed by surprise. Copy mode takes
the keyboard from the input line and from the input method, draws `-- COPY --` in
the bar, hides the caret and shows a cursor in the replies, so a swallowed
keystroke is never a mystery.

Editing is the usual set: `Left`, `Right`, `Home`, `End`, `Backspace`, `Delete`,
and `Ctrl+Left`/`Ctrl+Right` by word. The caret is drawn where the input holds it,
and a question longer than the bar scrolls under the caret rather than running off
the card.

Typing goes through `zwp_text_input_v3` when an input method is there and through
the keysyms when it is not — decided per keystroke, not once: a key pressed while
an input method is in play is held for the rest of that event round, and typed
itself if the input method says nothing about it. That is what keeps letters and
digits working after the input method is switched off.

## What it depends on

Seven crates, 59 in the whole tree including transitive ones — the same shape as
`wlmenu`, whose rendering core this borrows: `smithay-client-toolkit`,
`wayland-client`, `wayland-protocols`, `skrifa`, `ab_glyph_rasterizer`,
`fontdb`, `pulldown-cmark`, and nothing else. No HTTP or TLS crate: the provider
request is one `curl` process.

## Why it is built this way

The provider request runs inside `bwrap` with the network and nothing else — no
home directory, no session socket, no writable path, no inherited environment — so
a question cannot turn into local access. The API key is handed to `curl` on stdin
rather than in `argv`, so it is in no process table.

Rendering is software: `wl_shm` for the buffer, `skrifa` to read the font's
outlines, `ab_glyph_rasterizer` to turn them into coverage. The design tokens and
the 1.5×-row-height rule come from `plush` (through `wlmenu`, whose rendering core
this started as) so they read as the same product. The
layout core is new: wrapping, a caret inside the text, multi-line answers in
cards and a scrollback viewport are exactly what `wlmenu`'s single-line rows
cannot do.

The transport is a child process rather than a crate because `curl -N` already
streams, and a streaming HTTP client with TLS would be most of the dependency
graph for one request. Answers are rendered as a markdown subset — fenced code
blocks in a monospace face, inline code, bold, italic, headings, lists, quotes,
tables and the text of links — and everything outside it (task lists, raw HTML) is
drawn as the characters the model sent. A table is drawn in the monospace face with
its columns padded so they line up, measured by display width rather than character
count: CJK and emoji take the width their face gives them, and a combining mark adds
none. The header starts the column widths and a later row may only widen them, so a
streaming table does not jitter; a cell too wide wraps inside its column. A table too
narrow for its columns degrades to `header: value` lines rather than losing the right
side. Copying a table row gives TSV with the visual padding removed, so a spreadsheet
sees the cells. There is no
complex-script shaping. A character no installed face covers is drawn as a hollow
box, and a tab keeps four spaces' width; the first such miss adds the system's
script faces to the chain.

## Security

The provider request runs inside `bwrap`:

```
bwrap --clearenv --unshare-all --share-net --die-with-parent --new-session
      --ro-bind /usr /usr
      --symlink usr/lib /lib  --symlink usr/lib64 /lib64
      --symlink usr/bin /bin  --symlink usr/sbin /sbin
      --ro-bind /etc/resolv.conf /etc/resolv.conf   (and /etc/hosts,
      /etc/nsswitch.conf, /etc/host.conf, /etc/ssl, /etc/ca-certificates,
      each only if it exists)
      --proc /proc --dev /dev
      /usr/bin/curl -N -K -
```

Inside it there is no home directory, no `$XDG_RUNTIME_DIR`, no Wayland socket,
no configuration file, and no writable path; the environment is empty, so no
inherited variable — the credential included — is visible. The loader symlinks
are load-bearing: on a merged-`/usr` system, binding only `/usr` means nothing
executes at all. When the sandbox cannot start, no request is sent and no
unsandboxed `curl` is run.

The API key travels in the curl configuration on stdin (`-K -`), so the process
table shows `bwrap … curl -N -K -` and nothing more. Verified during a live
request: 58 processes inspected, none with the key in its `argv`.

What this does **not** cover:

- The network is reachable without restriction: the provider sees the question,
  and so does anything able to intercept the connection.
- A proxy you configure (`--proxy`, `ASIDE_PROXY`, `proxy` in the config) is the
  only way to route the request: the sandbox runs with an empty environment, so
  `http_proxy` and friends never reach it. An HTTPS target stays a TLS tunnel the
  proxy cannot read; a proxy that terminates TLS sees the question and the key.
- The credential is readable by any process running as the same user.
- The panel itself is not sandboxed; its own process reads fonts and the config.

## Privacy

The question, the request body, the response bytes and the rendered answer live
in memory. Nothing is written to disk.

Every path-opening call in the program, in full:

| Where | Call | Mode |
|---|---|---|
| `font.rs` | `fs::read` on the chain's font files | read |
| `provider.rs` | `read_to_string` on `config.toml` | read |
| `main.rs` | `fs::write` of the `--preview` images, and only under `--preview` | write |
| `control.rs` | `remove_file` of a stale socket in `$XDG_RUNTIME_DIR` (tmpfs), before binding a new one | unlink |

No state, cache or history directory is created or read, and no temporary file is
created by the transport: the curl configuration arrives on stdin and the
response is streamed straight into memory. A run with `XDG_CONFIG_HOME`,
`XDG_CACHE_HOME`, `XDG_STATE_HOME` and `XDG_DATA_HOME` pointed at an empty
scratch directory leaves that directory untouched.

Diagnostics go to standard error only and never contain the question, the answer
or the key. Whether the parent redirects standard error into a journal is outside
this program's control.

`Alt+C` is the one deliberate exit: after it, `wl-copy` serves the copied line to
anything that asks, and its process outlives the panel. The compositor, the
screen, and any process running as the same user can see or read the answer
regardless — the memory is not locked and the panel does not pretend otherwise.

A live checklist for the parts that need a real session is in
`docs/live-checklist.md`; the man page is `docs/aside.1`.
