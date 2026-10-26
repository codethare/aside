# Tasks

## 1. The output is chosen before the surface exists

- [x] 1.1 `--output NAME` is parsed, in `USAGE`, and passed to `run`
- [x] 1.2 A roundtrip before the first commit fills the output list; `choose_output` picks the named output, or the only one
- [x] 1.3 The layer surface is re-created on the chosen `wl_output`, and the chosen output's logical size is noted before the surface is committed
- [x] 1.4 An unknown name is one line naming the outputs that exist, and no panel
- [x] 1.5 A test covers `choose_output`: the exact name, the single output, the several-outputs case and the unknown name
- [x] 1.6 `surface_enter` is what says which output the panel is on; `new_output` notes nothing, and `update_output`/`output_destroyed` only act on that one, so a late report from another screen cannot re-centre the panel
- [x] 1.7 The spec says which single case may use the assumed height for one frame, and that another output's size MUST NOT overwrite the panel's

## 2. Docs and gate

- [x] 2.1 README, man page and the live checklist say what the flag does and what happens without it
- [ ] 2.2 A hand check on two outputs: `--output` pins it, no flag follows the compositor, and moving the pointer to the other screen does not move the panel
- [x] 2.3 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`
