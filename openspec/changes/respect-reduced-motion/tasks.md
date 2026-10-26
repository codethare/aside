# Tasks

## 1. One key, one helper

- [x] 1.1 `animations` is a known key, validated in `parse_config` like `font_size`, with a test for the accepted and refused values
- [x] 1.2 `State::animate` is read from the configuration at startup
- [x] 1.3 `clock_ms(animate, since)` returns the elapsed milliseconds, or a reading past every deadline when animation is off; the opening, the closing and a shape change all read it
- [x] 1.4 A test pins it: with animation off the tracks are already at their end and the closing has already finished; with it on they are not

## 2. Docs and gate

- [x] 2.1 README and the man page list the key, its values and what stays moving
- [ ] 2.2 A hand check: `animations = "false"`, summon, send, close — nothing grows, the light still runs on a slow request
- [x] 2.3 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `groff -man -Tutf8 docs/aside.1`
