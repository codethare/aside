# Keep the question in the bar

## Why

`Return` takes the question out of the bar and leaves an empty line behind. The
question is kept for the request, but it is drawn nowhere: the transcript holds the
answers, and a reader who wants to check what they actually asked — a typo, a wrong
name, the word they meant to add — has to remember it. While a slow model thinks, the
bar is the only thing on screen, and it is blank.

## What changes

The question stays in the bar after `Return`, for as long as the request runs.

- It is not typeable while it is there: no caret, no pre-edit, and no key reaches it.
  It is a record, not a line.
- If it is wider than the bar, it scrolls slowly from right to left; at its end it
  holds, turns round and comes back, and turns round again for as long as the request
  is unfinished.
- It goes when the request ends — answer, error or cancel — or as soon as the reader
  writes something else, which is what makes a follow-up a fresh line.

The transcript still draws no question, and nothing is written anywhere.

## Impact

- `src/panel.rs`: the bar takes the question and its age, `marquee` for the offset,
  and no caret while a question is out.
- `src/main.rs`: the question's text and when it was sent, set in `send`, dropped when
  the request ends or the line is typed into, and passed to `panel::draw` only in
  typing mode.
- `README`, `docs/aside.1`, `docs/live-checklist.md`, `CLAUDE.md`.
- No dependency, nothing on disk.
