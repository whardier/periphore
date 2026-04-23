---
phase: 02-identity-cryptography
verified: 2026-04-22T00:00:00Z
status: gaps_found
score: 9/10 must-haves verified
overrides_applied: 0
gaps:
  - truth: "Identicon display can be disabled via config or CLI flag, with word-phrase-only verification still functional"
    status: failed
    reason: "identity.show_identicon parses from TOML config correctly and defaults to true, but the flag is never read or acted upon in periphored/src/main.rs. GetIdenticon always returns the identicon regardless of show_identicon value. The SEC-04 requirement text says 'display can be disabled' — the flag exists in config but has zero behavioral effect."
    artifacts:
      - path: "crates/periphored/src/main.rs"
        issue: "config.identity.show_identicon is never referenced. GetIdenticon arm returns identity.identicon() unconditionally at line 143."
      - path: "crates/periphore-config/src/schema.rs"
        issue: "IdentityConfig.show_identicon is defined correctly and defaults to true, but is never consumed by the daemon."
    missing:
      - "Read config.identity.show_identicon in the GetIdenticon dispatch arm and return an empty identicon string (or IpcResponse::Ok) when false"
      - "Alternatively: read show_identicon at daemon startup and conditionally log the identicon"
      - "A test that sends GetIdenticon with show_identicon=false and asserts the identicon is suppressed"
human_verification:
  - test: "Cross-platform identicon visual identity check"
    expected: "The identicon rendered from an identical keypair (same 32-byte seed) produces character-by-character identical output on macOS and Linux"
    why_human: "Cannot test cross-platform identity programmatically in a single-platform CI run. The Drunken Bishop algorithm is deterministic pure Rust with no platform-conditional paths, so this is low risk, but ROADMAP SC3 specifically calls out 'both macOS and Linux'."
---

# Phase 2: Identity & Cryptography Verification Report

**Phase Goal:** Every node has a persistent Ed25519 cryptographic identity; the fingerprint is derived, displayed as an identicon and word phrase, and accessible via the IPC protocol. Identity is integrated into the daemon startup sequence.
**Verified:** 2026-04-22
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Running periphored for the first time creates a key file at the XDG data path | VERIFIED | `IdentityStore::load_or_create` with `create_dir_all` + `create_new(true)` wiring in `crates/periphore-identity/src/lib.rs:68-94`; `test_first_run_creates_key_file` passes |
| 2 | The key file is created with mode 0600 (no world-readable window) | VERIFIED | `OpenOptionsExt::mode(0o600)` with `create_new(true)` at lib.rs:84-89; atomic creation, no race window |
| 3 | Running periphored a second time loads the same keypair from the file | VERIFIED | `load_or_create` path.exists() branch at lib.rs:58-67; `test_load_after_create_is_identical` passes |
| 4 | A corrupt key file returns IdentityError::CorruptKeyFile | VERIFIED | `bytes.len() != 32` check at lib.rs:61-63; `test_corrupt_key_file_error` passes with `CorruptKeyFile(16)` |
| 5 | The SHA-256 fingerprint is deterministic (same seed -> same 64-char lowercase hex) | VERIFIED | SHA-256 of `verifying_key().to_bytes()` at lib.rs:133-136; `test_fingerprint_determinism` passes |
| 6 | periphored startup logs fingerprint at info level after identity load | VERIFIED | `tracing::info!(fingerprint = %identity.fingerprint_hex(), "identity loaded")` at main.rs:60-63 |
| 7 | GetStatus IPC response includes the real fingerprint_hex string | VERIFIED | `fingerprint: Some(identity.fingerprint_hex())` at main.rs:124; `get_status_returns_status_response` IPC test passes |
| 8 | identicon() returns an 11-line OpenSSH Drunken Bishop string with correct header/footer | VERIFIED | `drunken_bishop()` implemented at lib.rs:150-202; `test_identicon_borders`, `test_identicon_line_count`, `test_identicon_determinism` all pass |
| 9 | word_phrase() returns exactly 6 lowercase BIP39 words, deterministic | VERIFIED | `word_indices()` + `BIP39_WORDS` indexing at lib.rs:127-131; `test_word_phrase_determinism`, `test_word_phrase_validity` pass |
| 10 | Identicon display can be disabled via config or CLI flag (ROADMAP SC5 / SEC-04) | FAILED | `IdentityConfig.show_identicon` parses from TOML and defaults to true, but the flag is never read in periphored dispatch. `GetIdenticon` always returns `identity.identicon()` unconditionally regardless of config. |

**Score:** 9/10 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/periphore-identity/src/lib.rs` | IdentityStore, IdentityError, load_or_create, fingerprint_hex, identicon, word_phrase, default_key_path | VERIFIED | All exports present and substantive; no stubs remain |
| `crates/periphore-identity/src/bip39.rs` | BIP39_WORDS static &[&str; 2048] with compile-time assertion | VERIFIED | 2048 words from "abandon" to "zoo"; `assert!(BIP39_WORDS.len() == 2048)` at line 270 |
| `crates/periphore-identity/tests/identity.rs` | 9 test functions, all active (0 ignored) | VERIFIED | 9/9 pass: 4 SEC-01 + 3 SEC-02 + 2 SEC-03 tests |
| `crates/periphore-protocol/src/ipc.rs` | IpcResponse::Identicon and IpcResponse::WordPhrase variants | VERIFIED | Both variants present with correct serde tag attributes |
| `crates/periphore-config/src/schema.rs` | IdentityConfig with show_identicon: bool field | VERIFIED | Struct present with Default impl (show_identicon: true) |
| `crates/periphore-config/src/lib.rs` | IdentityConfig re-exported in pub use | VERIFIED | Line 15: `pub use schema::{..., IdentityConfig, ...}` |
| `crates/periphored/src/main.rs` | Identity loaded at startup, GetIdenticon/WordPhrase dispatch wired | PARTIAL | Identity loading: verified. Dispatch to real identity.identicon()/word_phrase(): verified. show_identicon gating: NOT wired. |
| `crates/periphore-protocol/tests/roundtrip.rs` | Round-trip tests for Identicon and WordPhrase | VERIFIED | Both variants in `ipc_response_all_variants_round_trip`; test passes |
| `crates/periphore-config/tests/config.rs` | identity_show_identicon_defaults_to_true + _can_be_disabled_via_toml | VERIFIED | Both config parsing tests pass (parsing only — not behavioral enforcement) |
| `Cargo.toml` | rand_core workspace dep with getrandom feature | VERIFIED | `rand_core = { version = "0.6", features = ["getrandom"] }` at line 44 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/periphored/src/main.rs` | `crates/periphore-identity/src/lib.rs` | `IdentityStore::load_or_create(&key_path)` | WIRED | Line 58; `default_key_path()` at line 56 |
| `crates/periphore-identity/src/lib.rs` | key file on disk | `OpenOptionsExt::mode(0o600)` at `create_new` | WIRED | Lines 81-89; atomic 0600 creation confirmed |
| `crates/periphored/src/main.rs select! arms` | `IpcResponse::Identicon / WordPhrase` | `identity.identicon()` and `identity.word_phrase()` calls | WIRED | Lines 141-151 |
| `IdentityStore::identicon` | `self.fingerprint [u8; 32]` | `drunken_bishop(&self.fingerprint)` | WIRED | lib.rs:119 |
| `IdentityStore::word_phrase` | `BIP39_WORDS static` | `word_indices(&self.fingerprint)` | WIRED | lib.rs:127-131 |
| `crates/periphore-protocol/tests/roundtrip.rs` | `IpcResponse::Identicon + WordPhrase` | `ipc_resp_round_trip` helper | WIRED | Lines 158-167 |
| `config.identity.show_identicon` | GetIdenticon dispatch behavior | Conditional in select! arm | NOT WIRED | `show_identicon` is never read in main.rs; identicon always returned |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `periphored GetIdenticon arm` | `identity.identicon()` | `drunken_bishop(&self.fingerprint)` pure function over Ed25519 keypair | Yes | FLOWING |
| `periphored GetWordPhrase arm` | `identity.word_phrase()` | `word_indices(&self.fingerprint)` + `BIP39_WORDS` static | Yes | FLOWING |
| `periphored GetStatus arm` | `identity.fingerprint_hex()` | SHA-256 of loaded/generated keypair | Yes | FLOWING |
| `IdentityConfig::show_identicon` | conditional identicon suppression | Never read by consumer | No | DISCONNECTED |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace compiles | `cargo build --workspace` | Exit 0 | PASS |
| All 9 identity tests pass | `cargo test -p periphore-identity` | 9/9 pass, 0 ignored | PASS |
| All 7 config tests pass | `cargo test -p periphore-config` | 7/7 pass | PASS |
| All 4 protocol round-trip tests pass | `cargo test -p periphore-protocol` | 4/4 pass | PASS |
| All 8 IPC socket tests pass | `cargo test -p periphore-ipc` | 8/8 pass | PASS |
| Workspace test total | `cargo test --workspace` | 30 tests, 0 failed | PASS |
| BIP39 compile-time assertion | Embedded in bip39.rs | `assert!(BIP39_WORDS.len() == 2048)` at line 270 | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| SEC-01 | 02-01 | Ed25519 keypair generation and persistence | SATISFIED | `IdentityStore::load_or_create` with 0600 key file; `test_first_run_creates_key_file`, `test_load_after_create_is_identical`, `test_corrupt_key_file_error`, `test_fingerprint_determinism` all pass; fingerprint in GetStatus |
| SEC-02 | 02-02, 02-03 | Identicon (Drunken Bishop) for visual verification | SATISFIED | `drunken_bishop()` implemented; `test_identicon_borders` (exact header/footer), `test_identicon_line_count` (11 lines), `test_identicon_determinism` pass; GetIdenticon IPC arm returns `IpcResponse::Identicon` |
| SEC-03 | 02-02, 02-03 | Word phrase (6 BIP39 words) for typed verification | SATISFIED | `word_indices()` + BIP39_WORDS; `test_word_phrase_validity` (6 lowercase words), `test_word_phrase_determinism` pass; GetWordPhrase returns `IpcResponse::WordPhrase` |
| SEC-04 | 02-03 | Identicon display can be disabled for headless setups | PARTIAL | `IdentityConfig.show_identicon` parses from TOML (two config tests pass), but the flag is NOT read by periphored. The config field exists and is documented, but has zero behavioral effect. ROADMAP SC5 requires the flag actually disables display. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/periphore-cli/src/lib.rs` | 16 | `anyhow::bail!("not yet implemented")` | Info | CLI stub — Phase 5 per ROADMAP; not blocking Phase 2 goal |
| `crates/periphored/src/main.rs` | 143 | `identity.identicon()` called unconditionally despite `show_identicon` config flag | Blocker | SEC-04 / ROADMAP SC5 not behaviorally enforced; the config flag is a no-op |

### Human Verification Required

#### 1. Cross-Platform Identicon Identity

**Test:** Generate an Ed25519 keypair with a fixed 32-byte seed on both macOS and Linux. Run the identicon function on the resulting fingerprint on each platform. Compare the output character-by-character.
**Expected:** The identicon strings are identical on both platforms (both header, all 9 grid rows, footer).
**Why human:** The Drunken Bishop algorithm is deterministic pure Rust with no platform-conditional branches, so this is a low-risk check, but ROADMAP SC3 explicitly requires verification on both platforms and cannot be automated in a single-machine run.

### Gaps Summary

One gap blocks full goal achievement: the `show_identicon` config flag is structurally wired into the config schema and parses correctly from TOML, but is never consulted in the daemon's IPC dispatch loop. `GetIdenticon` unconditionally calls `identity.identicon()` regardless of `config.identity.show_identicon`. ROADMAP Success Criterion 5 and SEC-04 both require the identicon display to actually be suppressible — not merely configurable in schema.

The fix is minimal: read `config.identity.show_identicon` in the `GetIdenticon` select! arm and return an empty identicon (or omit the identicon field, or return `IpcResponse::Ok`) when `false`. A test exercising this behavior path is also needed.

All other Phase 2 deliverables (SEC-01 keypair persistence, SEC-02 Drunken Bishop identicon, SEC-03 BIP39 word phrase, IPC protocol wiring, config schema) are fully and correctly implemented. The workspace compiles cleanly and 30 tests pass across all crates.

---

_Verified: 2026-04-22_
_Verifier: Claude (gsd-verifier)_
