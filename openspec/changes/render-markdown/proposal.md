# Render a markdown subset in the replies

## Why

Answers arrive as markdown and are drawn as literal text: `**bold**` shows its
asterisks, a fenced code block shows its backticks and loses the one thing code
needs — even columns. For a panel whose whole output is model prose, that is the
largest remaining gap between what the model sends and what the reader sees.

## What changes

The replies render a markdown subset:

- fenced code blocks — monospace, wrapped with a hanging indent, on a tinted
  background, with no language label drawn and no syntax highlighting;
- inline code — monospace, tinted;
- `**bold**` and `*italic*` — a second ink weight and a slanted face if the chain
  has one (no synthesised slant otherwise);
- headings — bold, with one empty row above them;
- unordered and ordered lists — a bullet or the number, hanging indent per nesting
  level;
- block quotes — indented, with a dim rail;
- links — the link text only, the URL dropped; images — their alt text.

Out of scope, and staying literal if a model sends them: tables, task lists, raw
HTML, footnotes, definition lists, strikethrough, math.

Parsing is `pulldown-cmark` with `Options::empty()`, so the options that would
enable wikilinks, tables, task lists and strikethrough are never on. The renderer
is ours: it produces the panel's own rows, not HTML.

## Dependency audit (asked for before adding)

`cargo add pulldown-cmark` with `default-features = false` resolves to four
packages, two of them already in this lock file:

| crate | version | licence | new? |
|---|---|---|---|
| pulldown-cmark | 0.13.5 | MIT | yes |
| unicase | 2.10.0 | MIT/Apache-2.0 | yes |
| bitflags | 2.13.2 | MIT/Apache-2.0 | no — already in the lock |
| memchr | 2.8.3 | MIT/Unlicense | no — already in the lock |

`default-features = false` drops `getopts` and the HTML escape crate; the parser
alone is `#![forbid(unsafe_code)]` unless the `simd` feature is enabled, which it
will not be.

Checks run, with results:

- **RustSec advisory database**: no advisory directory for any of the four crates,
  so none has a RUSTSEC id. `cargo audit` (advisory db refreshed) reports nothing
  for a probe crate with this dependency, and nothing for this repository's current
  69-crate lock, exit 0 in both cases.
- **0.13.4 and earlier do have public security problems**, and they are the reason
  the version is pinned rather than floated. Issue #1179 (opened by a downstream
  viewer's fuzzer, fixed by 0.13.5 on 2026-10-21) lists three:
  1. exponential event growth with `ENABLE_WIKILINKS` — a 112-byte document grows
     to 327k events, 16 GB RSS in their fuzzer. Not reachable here: wikilinks are
     off.
  2. a panic in `handle_wikilink` on malformed input — same, wikilinks off.
  3. **a panic in `into_offset_iter()` on 13 bytes with no options set**
     (`"- [o]:u\n    \t"`). This one applies to us: the offset iterator is exactly
     the API this change wants, and the input is model output, which is not
     trusted to be well-formed. Fixed in 0.13.5; verified locally — the repro
     runs on 0.13.5 and would have panicked on 0.13.4.
- No CVE was ever assigned; the project has no `SECURITY.md`, which is why the
  report went public as an issue instead of an advisory. So the pin is the control,
  not the advisory feed.
- No build-time surprises: `unicase` has no `build.rs`; `pulldown-cmark`'s is 276
  lines that compile to an empty `main` unless the `gen-tests` feature is on.
- Name confusion worth knowing: RUSTSEC-2023-0124 is about **`bit-flags`**, a
  typosquat removed from crates.io. The dependency here is `bitflags`, already in
  this tree via `smithay-client-toolkit`.
- Pathological input: the parser documents linear time with a cap on link nesting
  depth, and `REPLY_CAP` already bounds what reaches it.

The pin is `= 0.13.5` in `Cargo.toml`, so a bad release cannot arrive by `cargo
update`, and the panic repro becomes a test in our own suite so a downgrade fails
here rather than in a live panel.

## Cost

- One dependency (two new packages) against a rule that says six; this is the ask.
- A row stops being one string with one ink. Interaction is kept safe by leaving
  the row's plain text as the string the selection, the copy and the copy-mode
  cursor already index into: styles are an overlay, offsets do not move.
- The streaming wrap becomes "render the whole answer again per delta", which is
  O(n) per delta. `REPLY_CAP` bounds n, so this is a few hundred microseconds, and
  the frame cost stays per visible row.
- `README`, `docs/aside.1` and `CLAUDE.md` (known limits: "No Markdown") move in
  step.

## Impact

- `src/markdown.rs` (new): parse and lay out an answer into rows.
- `src/layout.rs`: `Row` gains its style spans, its indent and a code flag; the
  renderer wraps per segment rather than by one width.
- `src/panel.rs`: a row is drawn span by span; the selection band follows the
  spans' own columns; code rows get a tinted background; the quote rail.
- `src/transcript.rs`: a turn's rows come from the renderer.
- `src/font.rs`: a monospace chain beside the text chain.
- `Cargo.toml`: one dependency, pinned.
