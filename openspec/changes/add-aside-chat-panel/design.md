# Design

## Context

Greenfield crate `aside`, started in this repository at `/mnt/projects/workspace/sakr`.
The repository's earlier history (a `quickbar` bar/notification daemon, last
commit 2026-09-14) is abandoned; nothing from it is reused, and its worktree is
not on disk. Motivation is in `proposal.md - Why`; behaviour is in `specs/`.

The reference implementation for the rendering half is `/mnt/projects/workspace/wlmenu`
(Rust, `edition 2024`, `wl_shm` + `ab_glyph` + `fontdb`, ≈5000 lines in `src/`,
82 headless tests). The brief for this project lists, file by file, what is worth
porting and what is not; the plan follows that list rather than re-deriving it.

Constraints that shaped every decision below:

- **This machine**: Arch Linux, kernel 7.2.6, `bwrap` 0.12.0, `curl`, `wl-copy`,
  `jj`, Rust 1.98.1. Merged-`/usr` layout.
- **The design shell has no compositor**: `XDG_SESSION_TYPE=tty` and
  `WAYLAND_DISPLAY` is empty. Nothing about the panel can be run here. Everything
  below therefore pushes behaviour into seams that run headless — the SSE parser,
  the curl-config assembly, the sandbox arguments, wrapping and layout, the
  clipboard invocation, the no-disk audit.
- Contracts to keep, from the brief and `wlmenu`: four dependencies and no
  network crate; software rendering only; design tokens copied exactly; row height
  ≈1.5× the font size; 8 px spacing grid; spec-then-plan workflow; inline
  assertion tests and no test framework.

### Facts verified for this design, 2026-10-17

Not assumed; each was observed on this host or against the live provider.

| Fact | Evidence |
|---|---|
| `bwrap` cannot exec anything with only `/usr` bound: the loader lives at `/lib64/ld-linux-x86-64.so.2` | `bwrap --ro-bind /usr /usr -- /usr/bin/true` → `execvp: No such file or directory` |
| Adding `--symlink usr/lib /lib --symlink usr/lib64 /lib64 --symlink usr/bin /bin --symlink usr/sbin /sbin` fixes it, and `/home` is then absent | sandbox runs `true`; `ls /home` inside → `No such file or directory` |
| A sandboxed `curl` reaches the provider over TLS | `--unshare-all --share-net` + `/etc/resolv.conf`, `/etc/hosts`, `/etc/nsswitch.conf`, `/etc/ssl`, `/etc/ca-certificates` → HTTP 200 |
| The CA bundle needs its symlink target bound | `/etc/ssl/certs/ca-certificates.crt` → `../../ca-certificates/extracted/tls-ca-bundle.pem`; binding only `/etc/ssl` fails with `error adding trust anchors` |
| Sandbox start-up costs ≈16 ms | five `bwrap … true` runs in 82 ms total |
| `GET /models` is **unauthenticated** — it returns 200 with no key at all | so it cannot be used to check the auth header form, contrary to the brief's suggestion |
| `Authorization: Bearer <key>` works; a bogus key gives 401 `Invalid 'Authorization' header or token.` | `POST /chat/completions` → 200 and 401 |
| The stream uses **bare LF**, not CRLF | captured body has 10 `\n`, zero `\r`, and ends `data: [DONE]\n\n` |
| The first delta carries only `{"role":"assistant"}`; the last chunk carries `usage` and `sla_metrics` | captured frames |
| `deepseek/deepseek-v4.1-flash` exists and supports `/chat/completions` | live catalog, and a live streaming call with that model returned 200 |
| `smithay-client-toolkit` 0.21.1 has **no** text-input client | `src/seat/` holds only `input_method.rs`, `input_method_v3.rs` — the server side |
| `wayland-protocols` 0.32.13 exposes `wp::text_input::zv3` behind its `unstable` feature | `src/wp.rs:524` |
| CJK face present: `NotoSansCJK-Regular.ttc`, 19 MB, plus `NotoSans-Regular.ttf` | `/usr/share/fonts/noto-cjk/`, `/usr/share/fonts/noto/` |

## Goals / Non-Goals

**Goals:**

- A v1 that does the whole job (multi-turn, streaming, scroll, copy) with the
  smallest code that holds, reusing `wlmenu`'s proven primitives unchanged.
- Contain the provider process so that a question cannot become local access,
  and make that containment checkable from `argv` and from the process tree.
- Keep every non-visual behaviour testable without a compositor.
- Leave the design tokens and layout rules in one place so a shared crate can be
  extracted later, once the chat layout stops moving.

**Non-Goals:**

- Extracting a shared UI crate with `wlmenu`. The chat layout will keep changing;
  freezing an interface now means maintaining the wrong one in two repositories.
- Markdown, code blocks, tables, syntax highlighting.
- Egress filtering. The sandbox has the network unchanged; a proxy or netns is a
  later change with its own design.
- Complex-script shaping, and `wp_fractional_scale_v1` (integer `wl_output.scale`
  only, as `wlmenu` does).
- Any persistence, and any copy path other than the clipboard.
- Reusing the abandoned repository history.

## Decisions

### D1 — Port `wlmenu`'s font and render core; write a new layout core

Taken from `wlmenu` almost unchanged: `font.rs` (family resolution through
`fontdb`, lazy face loading, glyph cache, subpixel coverage planes — the font is
parsed by `skrifa` rather than `ttf-parser` since see tasks 14), `render.rs`
primitives (BGRA span painting, rounded-outline clipping, glyph blit, blend
helpers, the WCAG contrast test), and the shape of `feed.rs` (reader thread,
mutex queue, EOF flag, drained by the frame tick). `anim.rs` was ported and then
dropped: nothing here animates a size, because the window's size is the
compositor's to decide, so the module had no caller.

Written new: wrapping into rows, an in-row caret, a transcript viewport, line
highlight, and multi-line input. `wlmenu`'s renderer assumes one text line per
row with clipped overflow and an end-of-line caret; a chat needs all three of the
things it does not have, which is why the layout core is replaced rather than
extended. Not ported at all: `items.rs`, `desktop.rs`, and the `menu` state
machine.

The lazy CJK face matters here: the chain's CJK face is 19 MB on this host and
must not be read unless a glyph actually needs it, or the first frame pays for it.

### D2 — An `xdg-shell` toplevel, with `xdg-activation` for raising

A chat you leave open is a window, not an overlay. `wlmenu`'s layer-shell overlay
uses `KeyboardInteractivity::Exclusive`, which stops every other window taking
focus while it is mapped — correct for a launcher, wrong for a panel that sits
open beside your work. A toplevel is also movable and resizable, and survives
focus loss.

Alternatives rejected: layer-shell overlay (the focus problem above, and the
brief's §2.1); a layer-shell panel with `on-demand` keyboard interactivity
(avoids the exclusive-focus problem but still anchors to an edge and cannot be
moved, and river's layer-shell support is conditional).

`smithay-client-toolkit` 0.21.1 has `xdg::activation` in-tree, so raising the
window on a second launch needs no new dependency.

### D3 — Transport is a `curl` child process; no HTTP or TLS crate

`curl -N` streams onto stdout, which is exactly the shape the SSE reader wants,
and keeps the dependency list at `wlmenu`'s four plus `wayland-protocols` (already
in that tree through SCTK) for the text-input protocol.

Alternatives rejected: `ureq` + `rustls` (measured in an earlier round at roughly
70 crates for a comparable stack, and `ring` builds C in `build.rs`); hand-rolled
HTTP/1.1 over `rustls` (57 crates and ownership of chunked decoding, for a
program whose whole job is "hand a JSON body to a streaming client").

### D4 — The sandbox: `bwrap` around `curl`, with the key on stdin

One `bwrap` invocation wraps the one `curl` invocation:

```
bwrap --clearenv --unshare-all --share-net --die-with-parent --new-session
      --ro-bind /usr /usr
      --symlink usr/lib /lib  --symlink usr/lib64 /lib64
      --symlink usr/bin /bin  --symlink usr/sbin /sbin
      --ro-bind /etc/resolv.conf /etc/resolv.conf
      --ro-bind /etc/hosts /etc/hosts
      --ro-bind /etc/nsswitch.conf /etc/nsswitch.conf
      --ro-bind /etc/host.conf /etc/host.conf
      --ro-bind /etc/ssl /etc/ssl
      --ro-bind /etc/ca-certificates /etc/ca-certificates
      --proc /proc --dev /dev
      /usr/bin/curl -N -K -
```

`--unshare-all --share-net` gives the process the network and its own everything
else. `--clearenv` is not decoration: with this argv and without it, the child
inherits the panel's environment, credential and `HOME` included — measured with
`/usr/bin/env` inside the sandbox, which printed `COMMAND_CODE_API_KEY` and
`HOME`. Cleared, the credential exists only on stdin, and TLS still verifies
against curl's compiled-in bundle path (verified: HTTP 200 with `--clearenv` and
no `SSL_CERT_FILE` or `SSL_CERT_DIR` in the environment). The loader symlinks are
not optional on this host: without them nothing
executes at all (verified above). The two CA binds are both required because the
bundle is a symlink out of `/etc/ssl`.

The curl configuration — URL, `Authorization: Bearer …`, content type, timeouts
and the request body as `data-binary` — arrives on **stdin** (`-K -`), so the key
is in neither `bwrap`'s nor `curl`'s `argv`. That closes the weakness the brief
records for the dropped variant, where the key sat in the process table.

Alternatives rejected: `-H "Authorization: Bearer …"` in `argv` (world-readable
by any local user); `bwrap --setenv` for the key (lands in `bwrap`'s own `argv`,
same leak); a config file on disk (violates the whole point); passing the key over
a separate file descriptor with `/proc/self/fd/N` (works, but depends on
`/proc/self/fd` resolving inside a fresh mount and proc namespace, and on fd
inheritance through `bwrap`, for no benefit over stdin).

The escaping layer is the interesting part: the question text is embedded in a
curl config value inside a JSON string, so one tested function escapes for both
layers (`\` and `"` for JSON, then `\`, `"` and control characters for the curl
config), and rejects control characters outright.

Consequence to state plainly: stdin is the config, so the panel writes it from a
dedicated thread while another reads stdout, or a child that is slow to start
would deadlock the reader.

### D5 — The stream reader is one lenient state machine

Requirements are in `specs/provider-stream`. The shape: bytes buffered across
reads, split on `\n` with a trailing `\r` stripped, `data:` payloads only,
comment frames and payloads whose content is absent ignored, `[DONE]` ends the
answer, closing without it means truncated. The captured stream from the live call
above is the fixture for all of it, including the role-only opening delta and the
trailing `sla_metrics` chunk.

Deadlines are curl's, not a panel-side watchdog: `--connect-timeout 10`,
`--speed-limit 1 --speed-time 30` for a stream that stops delivering, and
`--max-time 300` as the upper bound. curl 8.22.0 has no time-to-first-byte
option, and both conditions exit 28, so the distinction the spec asks for comes
from the panel knowing whether any byte arrived: exit 28 with none is "the
provider sent nothing", with some is "the stream stopped". That needs no new
timer in the event loop, just a byte counter that already has to exist.

### D6 — IME: a hand-written `zwp_text_input_v3` client

SCTK has no client-side text input, so the panel owns the protocol directly using
`wayland_protocols::wp::text_input::zv3` (already reachable in the dependency
tree). Scope: `enable`/`disable` on keyboard enter and leave, `set_cursor_rectangle`,
handle `preedit_string`, `commit_string`, `delete_surrounding_text`, `done`; the
serial is held and acknowledged per `done`.

The panel degrades rather than fails: with no text-input manager advertised it
accepts raw keys and says IME input is unavailable (`specs/chat-panel`).

Rejected: skipping IME because it cannot be tested in this shell. A CJK typist
cannot use the tool without it, so it is the one part of the panel that must be
written blind and verified on the user's session from a checklist.

### D7 — Incremental layout, fixed viewport

The transcript owns a vector of already-wrapped rows; a streaming answer keeps a
tail buffer, and each drained delta wraps only the tail and appends whole rows
(`take_row`). Re-wrapping the transcript per delta is O(n²) and visibly janks; the
frame cost must stay proportional to the rows on screen. The viewport is at most
64 rows, mirroring `wlmenu`'s bound, which keeps the shm buffer bounded.

### D8 — Tokens, geometry and the working indicator

Exact values from the brief's token table: radius 12, `PAD` 16, top margin 32,
4 px prompt rail in `LABEL_ACCENT`, 8 px gap after the rail, 2 px pill padding,
1 px divider at `LABEL_ACCENT` alpha 64, `#222222`/`#bbbbbb` normal,
`#2e2e2e`/`#eeeeee` prompt, `#0b6285`/`#ffffff` selection. Row height stays
`size + 2 * round(size * 0.25)`. Content inset ≥ corner radius is asserted at
compile time and text contrast is asserted at ≥4.5:1, both as `wlmenu` does.

The working indicator is the 4 px rail, drawn in `BG_SEL` while a stream is open
and in `LABEL_ACCENT` otherwise. It changes a colour and no geometry, so the
transcript cannot jump when a stream ends — which is what the earlier round asked
for and what a spinner could not deliver.

Subpixel text filtering only at scale 1; grayscale above, where subpixel costs
35–55 % more for colour fringes nobody wants.

The brief's 32 px top margin is not carried over: it is a layer-shell margin
(`layer.set_margin`) and a toplevel has no such thing, so the window is placed by
the compositor and emulating the margin would only produce a lie in the token
table. Everything that shapes the panel's look — radius, `PAD`, the rail, the
pill padding, the divider, the colours, the 1.5× row height — is kept exactly.

The input is one logical line that soft-wraps, so nothing needs to decide what
`Up`/`Down` mean while the caret is in the input: they always move the transcript
highlight, and the caret moves with the standard editing keys. Dropping a newline
key is a deliberate simplification — a multi-line question can still be pasted —
and it is the cheapest thing here to put back if the user wants it.

### D9 — Single instance inverts `wlmenu`'s semantics

A unix socket in `$XDG_RUNTIME_DIR`, as in `wlmenu`'s `control.rs`, but the
message means *raise*: an instance that finds the socket takes the new process's
request, activates its own window with `xdg-activation`, and the new process exits
zero. A stale socket is detected and taken over. `wlmenu`'s opposite behaviour
(a second launch dismisses the open menu) is exactly what this must not do.

### D10 — Ephemerality by construction, verified by audit

There is no persistence code to disable: nothing opens a path for writing, the
config is read-only, the diagnostics go to stderr, and the transcript is a heap
vector plus the shm buffer. The check in `specs/ephemeral-conversation` is an
`strace -f -e trace=openat` audit of a full run, recorded in the run's notes,
plus the review of every `OPEN`-style call in the source. `memfd_secret`,
`MADV_DONTDUMP` and `PR_SET_DUMPABLE` from the earlier round's design are
deliberately **not** used: they claim more than they deliver (root and
`CAP_SYS_PTRACE` read the memory anyway, and the screen shows the answer), and
nothing asked for them. What is claimed is what a run can be shown to do.

### D11 — A headless preview mode, for want of a compositor

`aside --preview <file.ppm>` renders a scripted transcript through the same
layout and paint code into an offscreen buffer and writes a PPM. PPM because it
needs no encoder dependency and no protocol. This gives the layout, wrapping,
focus ring, highlight and token use a way to be looked at on a machine with no
Wayland session, and it is how the side-by-side comparison with `wlmenu` will be
prepared before the user checks it on their session.

### D12 — Copy goes to the clipboard only

`Alt+C` on the highlighted line runs `wl-copy` with that line on stdin. Printing
to stdout was considered and left out: the panel is normally started by a
compositor keybinding, where stdout goes nowhere, and the user asked for the
clipboard. `wl-copy` keeps a process alive to serve the selection — the one thing
that outlives the panel, and the reason `specs/ephemeral-conversation` names the
clipboard as the boundary.

### D13 — Geometry, and re-wrapping exactly once per resize

The window opens at 900×520 logical pixels with a minimum of 420×200, and then
follows the compositor's `configure` in both dimensions. The client does not
impose a maximum size and does not fight the compositor over size: a toplevel
that remembers a size the compositor did not configure is what produces the
`wlmenu` first-commit bug the brief warns about.

A width change invalidates wrapping, so the transcript is re-wrapped once for the
new width and the per-frame cost returns to the visible-row cost. The viewport is
bounded at 64 rows even when the window is taller. Incremental wrapping (D7) is an
optimisation that a resize throws away and rebuilds; keeping the transcript as
whole turns and re-wrapping on demand is what makes that a single O(n) step rather
than a second layout model.

### D14 — The config file is parsed without a parser dependency

`$XDG_CONFIG_HOME/aside/config.toml` holds flat `key = "value"` lines with `#`
comments — `model` and `base_url`, nothing else. That is small enough to parse in
around forty lines with the line number attached to every error the spec asks for,
which is the whole error-reporting requirement. The API key is not a config key at
all: it comes from the environment, so a config file can never hold a credential.

Alternatives rejected: the `toml` crate (about ten crates with the serde chain,
for a file with two scalar keys, and the project rule is to ask before adding a
dependency — avoiding the question is cheaper than asking it); `basic-toml` (three
crates, and the same objection at a smaller scale). If the config ever needs
tables or arrays, the answer changes to a real parser, and that is a change with
its own proposal.

## Risks / Trade-offs

- [The IME client is the only part that cannot be tested locally, and it is
  written blind] → keep its surface to the six calls above, degrade to raw keys
  when the global is absent, and put a fcitx/ibus step in the manual checklist
  the user runs on their session.
- [The `bwrap` recipe is host-specific: merged `/usr`, and the loader symlinks
  are load-bearing] → build the argv from a table whose entries are probed at
  start-up; if the sandbox cannot start, report it and send nothing
  (`specs/chat-sandbox`), rather than running `curl` outside it.
- [Curl config escaping has two layers and a mistake leaks a broken request or,
  worse, injects configuration] → one function, unit tests including quotes,
  backslashes, newlines, CJK and a 64 KiB body.
- [stdin carries the request body, so a slow child could deadlock a naive
  writer] → write from a dedicated thread; test with a deliberately slow reader.
- [A stream can outlive its usefulness: the provider may keep sending after the
  panel has all it will keep] → cancel by killing the child on `Esc` and on exit;
  the reply cap bounds what is retained.
- [The transcript grows with every turn and is never persisted] → per-answer cap
  of 64 KiB bounds the worst case; a whole-sitting bound is not built until a
  session is shown to need it.
- [Fractional scaling is out of scope, so a 1.5× output is whatever integer
  scale the compositor reports] → same behaviour as `wlmenu`, stated in the
  README rather than discovered later.
- [`wl-copy` leaves a resident process holding the copied line] → deliberate,
  and documented as the boundary of the guarantee.
- [Dropping the newline key is a user-visible call] → pasted text can still carry
  newlines; restoring `Shift+Enter` is one binding and one match arm, and the
  design says so here so the reversal is cheap.
- [`wlmenu`'s font-chain disk cache is not ported] → it wrote to
  `$XDG_CACHE_HOME`, which the no-disk requirement forbids, so the chain is built
  from a fixed candidate list (a stat per path) and only falls back to scanning
  the font database when none of those paths exists. The cost is the scan on an
  unusual machine; the cache never existed on this one.
- [Protocol errors turning into a spin: `wlmenu` had exactly this bug via a
  first-commit/configure size mismatch] → the first commit follows the last
  `configure`, `wl_display` errors are checked, and every error path exits.

## Migration Plan

None: greenfield, nothing deployed, no state to convert. Rollback is deleting
the binary. The abandoned `quickbar` history in this repository stays untouched
and unused.
