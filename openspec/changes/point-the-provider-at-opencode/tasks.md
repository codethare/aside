# Tasks
## 1. The variable is `OPENCODE_API_KEY`
- [x] 1.1 `provider::KEY_VAR` is `OPENCODE_API_KEY`, and the test that pins it asserts the new name
- [x] 1.2 Nothing reads the old name: no fallback, no second accepted variable
- [x] 1.3 The live exchange test and the sample failure text name the new variable
## 2. The default base URL is opencode's
- [x] 2.1 `provider::DEFAULT_BASE_URL` is `https://opencode.ai/zen/go/v1`, and the tests that pin the default and the curl URL assert it
- [x] 2.2 The sandbox's network test reaches that host, so the default provider's DNS and TLS are what gets exercised
- [x] 2.3 The model default is unchanged, and opencode lists it as a canonical id
## 3. The documents say the same
- [x] 3.1 `README.md` and `docs/aside.1` show `export OPENCODE_API_KEY=...`
- [x] 3.2 `docs/live-checklist.md` names the new variable in its setup and in its unset-key check
- [x] 3.3 `aside --help` and `docs/aside.1` name the new default base URL
- [x] 3.4 The gate passes: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings`
