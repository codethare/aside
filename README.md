# aside

A keyboard-summoned AI chat panel for Wayland. Press a key, type a question, watch
the answer stream in, ask a follow-up, take a line out to the clipboard. The
transcript lives in this process's memory and dies with it: no history, no cache,
no log file.

Behaviour is specified in `openspec/changes/add-aside-chat-panel/` and
`openspec/changes/add-panel-states/`. Those directories
is the source of truth for what the tool does; this file only says how to run it
and why it is built the way it is.

## Run

```sh
aside
```

It needs a Wayland session, and the programs on `PATH`: `bwrap`, `curl`,
`wl-copy` and `wl-paste` (both from `wl-clipboard`). Bind it in your compositor,
for example in Hyprland:

```
bind = SUPER, A, exec, aside
```

Press the key again while it is open and the running panel is activated rather than
duplicated.

The panel is an ordinary window: its `app_id` is `aside`, and where it sits, whether
it floats and when it has the keyboard are the compositor's business. A tiling
compositor needs a float rule, and the panel is happiest near the bottom of the
screen:

```
# sway
for_window [app_id="aside"] floating enable, resize set 640 500, move position 640 560, focus
# niri (config.kdl)
window-rule {
    match app-id="aside"
    open-floating true
}
# river
riverctl float-filter-add app-id aside
```

Configuration is optional: `~/.config/aside/config.toml` with flat `key = "value"`
lines, and five keys — `model`, `base_url`, `proxy`, `font_size` and `font_family`:

```toml
model = "deepseek/deepseek-v4.1-flash"
font_size = "24"                 # 8 to 72 logical px; the bar follows it
font_family = "Noto Sans Mono"    # a name nothing provides falls back to the chain
```

A size that is not a size is an error naming the file and the line, and nothing
opens. The credential is environment-only:

```sh
export COMMAND_CODE_API_KEY=...
```

Flags override both: `--model`, `--base-url`, `--proxy`. `aside --preview out.ppm` renders a
scripted transcript to an image without opening a window, which is how the layout
is checked on a machine with no compositor.

## What it looks like

A window with no decorations, focused by the compositor the way any window is, so
neither appearing nor being summoned again costs a click; click another window and
it keeps the keyboard, click the panel and it takes it back. The chrome is
`plush`'s: a 26 px drop shadow measured from each
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
gaps. A question you sent is kept for the next request and drawn nowhere.
`Up`/`Down` mark the highlighted line with brighter ink two pixels in from its
neighbours, so there is no mark in the margin at all. The replies arrive with the
panel's own growth and are not animated a second time; a follow-up's answer simply
appears. The wheel scrolls the replies while the pointer is over them — three lines a notch, a touchpad's own
delta otherwise — and does nothing over the bar.

The sizes are the panel's own, and the window is exactly that size: the shadow
margin is inside it. The answering panel is a **fixed** size: the bar, the 16 px
between the cards, and a box 38 % of the output height, capped by the 64 rows one
frame may draw. It does not grow as the answer does — a longer answer scrolls
inside the box and shows a 3 px scroll indicator on the right edge. Nothing the
panel does moves the window: the compositor's own move binding and its window rules
are where a position comes from, and no position is ever written down. Dragging
across the replies selects text instead — press on a line, drag, release — and
`Alt+C` or `Ctrl+C` copies what is selected, one line per line of it. `Esc` clears a
selection before it closes the panel.
State changes are animated, and always as one card changing shape, painted inside
one buffer, so the surface is resized once per state and never per frame. Opening
takes 520 ms: a line spreads out of a point at the centre, and once it has the
panel's width the card unfolds up out of it. Sending a question moves nothing: the
bar stays, and a light runs round its rim — one lap in 1.4 s — until the first text
arrives, which grows the panel out of the bar in 460 ms — that growth is the reply's
entrance, and nothing plays after it. A follow-up keeps the box it was asked from:
the panel does not fold shut and open again, the light simply runs on its bar's rim
until the new answer starts. Closing is the opening
run backwards, over 460 ms: the card rolls up into a line, the line pulls in from
both ends, and only then does the process exit.

## Keys

| Key | Mode | Action |
|---|---|---|
| `Enter` | typing | send |
| `Esc` | typing | cancel a streaming answer, or close the panel |
| `Esc` | prefix | leave the mode, cancelling nothing |
| `Esc` | copy | clear the selection; leave copy mode when there is none |
| `Ctrl+T` | typing | enter the prefix mode; the next key is a settings command |
| `[` | prefix | enter copy mode |
| `Up` / `Down` | typing | move the highlighted transcript line |
| `PageUp` / `PageDown` | typing | move it a page |
| `Alt+C` / `Ctrl+C` | typing, copy | copy the selection, or the highlighted line |
| `Ctrl+V` | typing | paste the clipboard into the input line |

Copy mode is tmux's, cut down to what the panel has to walk. `h`/`l` move a
character, `0`/`$` to the ends of the line, `j`/`k` a line, `w`/`b` a word,
`Ctrl+d`/`Ctrl+u` half a page, `Ctrl+f`/`Ctrl+b` (or `PageDown`/`PageUp`) a page,
`g`/`G` the first and last line. `Space` starts a selection and `Enter` or `y`
copies it — or the line the cursor is on — and leaves; `q` leaves, `Esc` clears a
selection first. After a copy the bar says `copied N lines` for a moment.

`Ctrl+T` is tmux's prefix shape rather than a mode toggle: it takes exactly one
key, and `[` is the only command it has so far — later settings commands belong
there. While it waits, the bar takes no input, changes tone and reads `press [ to
enter copy mode`, so the next key is never swallowed by surprise. Copy mode takes
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

Six crates, 125 in the whole tree including transitive ones — the same shape as
`wlmenu`, whose rendering core this borrows: `smithay-client-toolkit`,
`wayland-client`, `wayland-protocols`, `skrifa`, `ab_glyph_rasterizer`,
`fontdb`, and nothing else. No HTTP or TLS crate: the provider request is one
`curl` process.

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
graph for one request. Answers are plain wrapped text: no Markdown, code blocks or
tables, and no complex-script shaping.

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
| `main.rs` | `fs::write` of the `--preview` image, and only under `--preview` | write |
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
