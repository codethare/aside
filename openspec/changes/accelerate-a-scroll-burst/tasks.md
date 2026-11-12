# Tasks

## 1. The burst

- [x] 1.1 `burst_factor`: `1 + 0.5 (n - 1)`, capped at four — verify: a test pins 1 → 1×, 2 → 1.5×, 7 → 4×, 50 → 4×, and that it never falls as the step count rises
- [x] 1.2 `burst_step` and `Burst::take`: a wheel notch one step per notch, a touchpad one step per gesture (its events within 150 ms are the same gesture), a pause of 150 ms (wheel) or 400 ms (touchpad) starting the count again — verify: a test runs a spin of notches 100 ms apart and reads 1, 2, 3, then leaves 200 ms and reads 1; and slides ten events 10 ms apart and reads 1 throughout, then a new gesture 200 ms later reads 2, another reads 3, and a 500 ms pause reads 1
- [x] 1.3 Multiply the wheel's distance by the burst's factor — verify: a test asserts one notch is 150 px of five 30 px rows and that a burst's second step is 225 px; the `Ctrl` size chord is a separate function that never reads the burst
- [x] 1.4 A source or stop notice asks for no distance and must not count as a step — verify: a test asserts nothing is a step at 0.0 or -0.0 and that a tenth of a pixel is

## 2. Docs

- [x] 2.1 `README.md`, `docs/aside.1`, `docs/live-checklist.md`: a quick burst travels farther, a pause returns a notch to five rows, `Ctrl` never accelerates — verify: each names the acceleration and the pause, and the checklist has steps a person can run with a wheel and with two fingers

## 3. Integration

- [ ] 3.1 Run the gate — verify: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings` are clean. Waiting on a machine with the system faces installed: this sandbox has no monospace face, so four font-metric tests fail before and after the change, and they are the only failures
- [ ] 3.2 Run a real session — verify: one notch is five rows, a spin of ten notches accelerates and stops at four times, a pause brings the next notch back to five rows, one two-finger slide stays 1:1 while three quick flicks grow, and `Ctrl` + the wheel resizes at one step a notch however fast it turns
