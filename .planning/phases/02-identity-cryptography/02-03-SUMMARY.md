---
phase: 02-identity-cryptography
plan: 03
subsystem: identity, protocol, config
tags: [ipc, identicon, word-phrase, sec-02, sec-03, sec-04, drunken-bishop, bip39, identity-config]

# Dependency graph
requires:
  - phase: 02-01
    provides: "IdentityStore with load_or_create(), fingerprint_hex(), identicon(), word_phrase()"
  - phase: 02-02
    provides: "identicon() and word_phrase() fully implemented (Drunken Bishop + BIP39)"

provides:
  - "IpcResponse::Identicon { fingerprint_hex: String, identicon: String } variant in periphore-protocol"
  - "IpcResponse::WordPhrase { words: Vec<String>, phrase: String } variant in periphore-protocol"
  - "IdentityConfig { show_identicon: bool } struct in periphore-config (defaults true, SEC-04)"
  - "Config::identity field wired with #[serde(default)] in periphore-config"
  - "IdentityConfig re-exported from periphore-config pub use"
  - "periphored dispatches GetIdenticon and GetWordPhrase with real identity data in select! arms"
  - "periphore-ipc test router updated to return IpcResponse::Identicon/WordPhrase stubs"
  - "Round-trip tests for Identicon and WordPhrase variants"
  - "Identity config tests: show_identicon defaults true, TOML can disable (SEC-04)"

affects:
  - 05-cli-tool
  - 06-tcp-peering

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "IpcResponse tag enum extension: new variants added after Peers, before Ok — preserves serde tag order"
    - "IdentityConfig with custom Default impl (show_identicon: true) — mirrors LoggingConfig pattern"
    - "send_ok() wildcard arm: stubs removed after moving to dedicated select! arms; compiler enforces exhaustiveness"
    - "IPC test router stubs use real IpcResponse variants to keep test compilation correct after protocol changes"

key-files:
  created: []
  modified:
    - "crates/periphore-protocol/src/ipc.rs — IpcResponse::Identicon and IpcResponse::WordPhrase variants added"
    - "crates/periphore-protocol/tests/roundtrip.rs — Identicon and WordPhrase round-trip cases added"
    - "crates/periphore-config/src/schema.rs — IdentityConfig struct + Config::identity field"
    - "crates/periphore-config/src/lib.rs — IdentityConfig added to pub use"
    - "crates/periphore-config/tests/config.rs — 2 new identity config tests"
    - "crates/periphored/src/main.rs — GetIdenticon/GetWordPhrase dispatch moved to select! arms; stubs removed from send_ok()"
    - "crates/periphore-ipc/tests/socket.rs — handle_test_command updated to return IpcResponse::Identicon/WordPhrase"

key-decisions:
  - "IpcResponse::Identicon and WordPhrase placed after Peers and before Ok — preserves existing variant ordering while grouping identity responses together"
  - "IdentityConfig cannot be set via PERIPHORE_IDENTITY_SHOW_IDENTICON env var (Figment split('_') produces wrong 3-level key path) — documented in code comment, TOML-only (T-02-15)"
  - "GetIdenticon and GetWordPhrase moved from send_ok() to dedicated select! arms so they have access to identity variable — the send_ok() helper has no access to identity"

requirements-completed:
  - SEC-01
  - SEC-02
  - SEC-03
  - SEC-04

# Metrics
duration: 3min
completed: 2026-04-23
---

# Phase 2 Plan 03: IPC Protocol Wiring Summary

**IpcResponse::Identicon and IpcResponse::WordPhrase variants wired end-to-end: protocol enum extended, IdentityConfig (SEC-04) added to config, periphored dispatches real identity data from select! arms**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-04-23T04:44:18Z
- **Completed:** 2026-04-23T04:47:16Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- `IpcResponse::Identicon { fingerprint_hex, identicon }` and `IpcResponse::WordPhrase { words, phrase }` added to `periphore-protocol/src/ipc.rs` with full serde tag support — serialize as `{"type":"identicon",...}` and `{"type":"word_phrase",...}`
- `IdentityConfig { show_identicon: bool }` added to `periphore-config/src/schema.rs` with `Default` impl (true); `Config::identity` field wired with `#[serde(default)]`; `IdentityConfig` re-exported from lib.rs pub use
- `periphored` `select!` loop: `GetIdenticon` arm calls `identity.fingerprint_hex()` and `identity.identicon()`; `GetWordPhrase` arm calls `identity.word_phrase()` and joins words; stub arms removed from `send_ok()`
- `periphore-ipc` test router updated so `GetIdenticon` and `GetWordPhrase` return proper `IpcResponse::Identicon` and `IpcResponse::WordPhrase` stubs (previously returned `IpcResponse::Ok`, which would be a type mismatch in client code)
- All 30 workspace tests pass: 4 protocol, 7 config (5 existing + 2 new identity), 9 identity, 8 IPC socket, 2 periphore binary

## Task Commits

1. **Task 1: IpcResponse variants, IdentityConfig, and all tests** — `ad00b90` (feat)
2. **Task 2: periphored dispatch + IPC test router** — `5e1f1d9` (feat)

## Files Created/Modified

- `crates/periphore-protocol/src/ipc.rs` — `IpcResponse::Identicon` and `IpcResponse::WordPhrase` variants with doc comments (SEC-02/SEC-03/D-09/D-10)
- `crates/periphore-protocol/tests/roundtrip.rs` — Two new cases in `ipc_response_all_variants_round_trip`
- `crates/periphore-config/src/schema.rs` — `IdentityConfig` struct with `Default` impl; `identity` field on `Config`
- `crates/periphore-config/src/lib.rs` — `IdentityConfig` added to `pub use schema::{...}` re-export line
- `crates/periphore-config/tests/config.rs` — `identity_show_identicon_defaults_to_true` and `identity_show_identicon_can_be_disabled_via_toml` tests
- `crates/periphored/src/main.rs` — `GetIdenticon` and `GetWordPhrase` select! arms with real identity calls; stubs removed from `send_ok()`; comment updated
- `crates/periphore-ipc/tests/socket.rs` — `handle_test_command` updated: `GetIdenticon` returns `IpcResponse::Identicon`, `GetWordPhrase` returns `IpcResponse::WordPhrase`

## Decisions Made

- `GetIdenticon` and `GetWordPhrase` had to be moved from `send_ok()` to dedicated `select!` arms because `send_ok()` is a free function with no access to the `identity` variable — this is the only structurally correct location for real dispatch
- `IdentityConfig::show_identicon` documented as TOML-only (not env-var configurable) due to Figment's `split("_")` producing a 3-level key path for underscore-bearing field names (T-02-15)

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None — all identity IPC wiring is fully implemented. `GetIdenticon` returns real `identity.identicon()` output; `GetWordPhrase` returns real `identity.word_phrase()` output.

## Threat Surface Scan

No new network endpoints or auth paths introduced. Changes are purely within the existing Unix domain socket IPC boundary (0600 permissions, same-user access only). Threat register items T-02-12, T-02-13, T-02-14, T-02-15 all mitigated as designed:
- T-02-12: Identicon is public-key-derived (not secret); socket remains 0600
- T-02-13: `send_ok()` wildcard arm satisfies exhaustiveness after stub removal; comment updated
- T-02-14: serde tag enum rejects unknown type strings at deserialization
- T-02-15: `show_identicon` env var confusion documented in code comment

## Phase 2 Requirements Status

| Requirement | Status | Delivered in |
|-------------|--------|-------------|
| SEC-01: Ed25519 keypair generation and persistence | Complete | 02-01 |
| SEC-02: GetIdenticon returns Drunken Bishop identicon | Complete | 02-02 + 02-03 |
| SEC-03: GetWordPhrase returns 6 BIP39 words | Complete | 02-02 + 02-03 |
| SEC-04: identity.show_identicon = false disables identicon | Complete | 02-03 |

Phase 2 (Identity & Cryptography) is fully complete.

## Self-Check: PASSED

| Item | Status |
|------|--------|
| `crates/periphore-protocol/src/ipc.rs` contains `Identicon` | FOUND |
| `crates/periphore-protocol/src/ipc.rs` contains `WordPhrase` | FOUND |
| `crates/periphore-config/src/schema.rs` contains `IdentityConfig` | FOUND |
| `crates/periphore-config/src/schema.rs` contains `show_identicon` | FOUND |
| `crates/periphore-config/src/lib.rs` re-exports `IdentityConfig` | FOUND |
| `crates/periphored/src/main.rs` has `IpcResponse::Identicon` in select! arm | FOUND |
| `crates/periphored/src/main.rs` has `IpcResponse::WordPhrase` in select! arm | FOUND |
| `crates/periphored/src/main.rs` has NO `IpcResponse::Ok` for GetIdenticon/GetWordPhrase | CONFIRMED |
| Commit ad00b90 (Task 1) | FOUND |
| Commit 5e1f1d9 (Task 2) | FOUND |
| `cargo test --workspace` exits 0 | VERIFIED |
| `cargo test -p periphore-protocol -- ipc_response_all_variants_round_trip` passes | VERIFIED |
| `cargo test -p periphore-config -- identity_show_identicon` passes (2 tests) | VERIFIED |

---
*Phase: 02-identity-cryptography*
*Completed: 2026-04-23*
