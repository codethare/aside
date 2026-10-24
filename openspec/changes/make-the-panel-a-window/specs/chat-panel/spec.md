# Spec Delta

## MODIFIED Requirements

### Requirement: The keyboard contract

The panel SHALL give these keys these meanings, and nothing else:

| Key | Mode | Action |
|---|---|---|
| `Enter` | typing | send the typed question |
| `Esc` | typing | cancel an open stream; clear a pointer selection; close the panel when neither is open |
| `Esc` | prefix | leave prefix mode, doing nothing else |
| `Esc` | copy | clear the selection; leave copy mode when there is none |
| `Ctrl+T` | typing | enter the prefix mode |
| `[` | prefix | enter copy mode |
| `Up` / `Down` | typing | move the highlighted transcript line |
| `PageUp` / `PageDown` | typing | move it one page |
| `Alt+C` / `Ctrl+C` | typing, copy | copy the selection, or the highlighted line |
| `Ctrl+V` | typing | paste the clipboard into the input line |
| copy-mode keys | copy | as the copy-mode requirement lists them |
| any other printable key | prefix, copy | nothing: the input line does not change |

The input SHALL be one logical line that soft-wraps, edited with the standard
keys. There SHALL be no newline key. Every action SHALL be reachable without a
pointer, and every pointer action SHALL have a keyboard equivalent. Moving the
panel is not one of the panel's actions: the compositor moves windows.

#### Scenario: Nothing moves the panel

- **WHEN** the user presses `Alt+h`, `Alt+Ctrl+h`, or drags the pointer across the panel
- **THEN** the panel's content does not move, the input line is unchanged, and the
  compositor still owns where the window is

#### Scenario: Escape still means what it meant

- **WHEN** the user presses `Esc` in typing mode while deltas are still arriving
- **THEN** the stream is cancelled, the panel stays open

## ADDED Requirements

### Requirement: The panel is a window the compositor places

The panel SHALL be an `xdg-toplevel` with `app_id` and title `aside`, and SHALL
request no decorations. Where it is, whether it floats, which workspace it is on
and when it has the keyboard SHALL all be the compositor's decisions: the panel
SHALL NOT set margins, anchors or a layer, and SHALL NOT move itself. The panel
SHALL be sized by the phase it is in, by attaching a buffer of that size, and the
compositor's configure SHALL be treated as permission to attach rather than as a
size to obey.

Focus SHALL arrive the way it arrives for any other window, so no click is needed
to type after the panel appears, and the compositor's own activation (a second
launch, a launcher, a window rule) SHALL reach the panel.

A close request from the compositor SHALL end the process, so a window the
compositor asked to close never leaves a process behind with no way to be seen.

#### Scenario: Summoned from a keybinding

- **WHEN** a compositor keybinding runs `aside`
- **THEN** a window with `app_id` `aside` appears, the compositor focuses it, and
  typing reaches the input line without a click

#### Scenario: Summoned again while it is open

- **WHEN** `aside` is run again while a panel is up
- **THEN** the running panel is activated through the compositor, and it is the one
  that has the keyboard

#### Scenario: The compositor closes it

- **WHEN** the compositor asks the window to close
- **THEN** the panel exits, and nothing is left running

#### Scenario: The panel never moves itself

- **WHEN** any key or pointer event arrives
- **THEN** nothing the panel does changes where its window is

### Requirement: The reply box is a share of the output height

The answering panel's replies card SHALL be 38 % of the output height, capped by
the 64 rows one frame may draw, and fixed for as long as the panel is answering:
the same size for one line and for a thousand. The replies scroll inside it.

#### Scenario: One size for every answer

- **WHEN** a second answer arrives in the same panel
- **THEN** the size does not change and the replies scroll inside the card they had

## REMOVED Requirements

### Requirement: The panel is a layer surface in three states

**Reason**: A layer surface cannot be focused by the compositor or by any window
rule, so the panel could only be typed into after a click. The panel is an
`xdg-toplevel` now.

**Migration**: Same three states and the same chrome, drawn in a window the
compositor places; `app_id` is `aside`, so window rules and float rules match it.

### Requirement: The panel is summoned, focused, and movable on both axes

**Reason**: The compositor moves windows. The drag, and the chord that stepped the
window, competed with the window manager and only existed because a layer surface
is placed by its own margins.

**Migration**: Placement is a float rule and the compositor's own move binding; the
keyboard contract no longer has `Alt+hjkl`.

### Requirement: The answer's height is capped by the bar's gap

**Reason**: There is no anchored gap under a window.

**Migration**: The box is 38 % of the output height, still capped at 64 rows.
