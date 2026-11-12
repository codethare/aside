# Tasks

## 1. The wheel's region ends at the card

- [x] 1.1 `over_replies` ends at the replies card's bottom edge — the bar's gap, the bar, and the shadow margin below it take no scroll — verify: a test pins the last row of the card as the region's last row, and asserts that the gap above the bar, the bar's first row, its last, and the margin below it are all refused
- [x] 1.2 Nothing else moves: the card, the gap and the bar are drawn where they were, and the probe for a reply line still measures from the card's top — verify: the panel's frame tests are unchanged

## 2. Integration

- [ ] 2.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean. Waiting on a machine with the system faces installed: this sandbox has no monospace face and no CJK face, so four font-metric tests fail before and after the change, and they are the only failures
- [ ] 2.2 Run a real session — verify: with an answer streaming, rest the pointer on the bar and flick the wheel and two fingers: nothing moves and the answer stays at its newest line, then arrives at the bottom when it ends. Over the replies, a wheel still scrolls and still stops following, and a scroll on the replies while the answer streams comes back to the bottom when the answer ends
