# Tasks
## 1. The request carries a routing key
- [x] 1.1 `provider::session_id` is made once per process and reused for every turn
- [x] 1.2 `curl_config` sends it as `x-opencode-session`, escaped like the other values, in the curl configuration rather than in `argv`
- [x] 1.3 A test asserts the header line holds the id, that two configurations from one process carry the same one, and that the id is not empty
- [x] 1.4 Every request sends `User-Agent: aside/<version>`, and a test pins that line
## 2. Verified on the wire
- [x] 2.1 Without the header the endpoint answers `400 MissingSessionID`; with it, authentication is the next gate — measured with a stand-in credential, since the real one is not in this checkout
- [x] 2.2 The gate passes: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings`
