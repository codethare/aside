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

It needs a Wayland session, and four programs on `PATH`: `bwrap`, `curl`,
`wl-copy`, and a compositor that speaks `xdg-shell`. Bind it in your compositor,
for example in Hyprland:

```
bind = SUPER, A, exec, aside
```

Press the key again while it is open and the running panel is raised rather than
duplicated.

Configuration is optional: `~/.config/aside/config.toml` with flat `key = "value"`
lines, `model`, `base_url` and `proxy` only. The credential is environment-only:

```sh
export COMMAND_CODE_API_KEY=...
```

Flags override both: `--model`, `--base-url`, `--proxy`. `aside --preview out.ppm` renders a
scripted transcript to an image without opening a window, which is how the layout
is checked on a machine with no compositor.

## What it looks like

A `wlr-layer-shell` surface on the overlay layer, anchored to the bottom of the
output, 32 px up and centred: the same panel `wlmenu` draws, 640 px wide and one
row tall (30 px at 20 px text), with the replies growing upwards above the input
bar. The bottom anchor is what keeps the bar in one place on screen; a top anchor
would carry it downwards by a row per wrapped line. The frame and the bar are
`wlmenu`'s too: a 24 px drop shadow, a premultiplied translucent body, a hairline
border and antialiased corners. It has three states:

| State | Panel size | What is drawn |
|---|---|---|
| idle | 640 × 30 | the bar alone |
| thinking | 60 × 60 | a circle with four squares, one lit every 200 ms |
| answering | 640 × (replies + bar) | the replies above, the bar at the bottom |

Only the model's text is on screen: an answer, a failure and the panel's own
notes are each drawn inside a rounded card inset by `PAD`, with 8 px of padding
above and below the text, and blocks are separated by an 8 px gap. A question you
sent is kept for the next request and drawn nowhere. `Up`/`Down` mark the
highlighted line with a 2 px rail in the left margin instead of a band across the
row.

The sizes are the panel's own; the layer surface adds the shadow margin on every
side and sits 8 px from the screen edge, so the panel's visible gap stays 32 px.
The answering panel never grows past the output height minus 64 px (the margin
above and the breathing room below); a longer answer scrolls inside it and shows
a 3 px scroll indicator on the right edge. States snap: there is no height tween.

## Keys

| Key | Action |
|---|---|
| `Enter` | send |
| `Esc` | cancel a streaming answer, or close the panel |
| `Up` / `Down` | move the highlighted transcript line |
| `PageUp` / `PageDown` | move it a page |
| `Alt+C` | copy the highlighted line to the clipboard |

Editing is the usual set: `Left`, `Right`, `Home`, `End`, `Backspace`, `Delete`,
and `Ctrl+Left`/`Ctrl+Right` by word.

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
the 1.5×-row-height rule come from `wlmenu` so the two read as the same product. The
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
