---
phase: 02-identity-cryptography
verified: 2026-04-22T12:00:00Z
status: passed
score: 10/10 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 9/10
  gaps_closed:
    - "Identicon display can be disabled via config (show_identicon gating now wired in GetIdenticon dispatch)"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Cross-platform identicon visual identity check"
    expected: "The identicon rendered from an identical keypair (same 32-byte seed) produces character-by-character identical output on macOS and Linux"
    why_human: "Cannot test cross-platform identity programmatically in a single-platform CI run. The Drunken Bishop algorithm is deterministic pure Rust with no platform-conditional paths, so this is low risk, but ROADMAP SC3 specifically calls out 'both macOS and Linux'."
---

# Phase 2: Identity & Cryptography Verification Report

**Phase Goal:** Every node has a persistent Ed25519 cryptographic identity; the fingerprint is derived, displayed as an identicon and word phrase, and accessible via the IPC protocol. Identity is integrated into the daemon startup sequence.
**Verified:** 2026-04-22
**Status:** human_needed
**Re-verification:** Yes — after gap closure (plan 02-04 closed the single gaps_found item)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Running periphored for the first time creates a key file at the XDG data path | VERIFIED | `IdentityStore::load_or_create` with `create_dir_all` + `create_new(true)` at lib.rs:71-89; `test_first_run_creates_key_file` passes |
| 2 | The key file is created with mode 0600 (no world-readable window) | VERIFIED | `OpenOptionsExt::mode(0o600)` with `create_new(true)` at lib.rs:84-89; atomic creation, no race window |
| 3 | Running periphored a second time loads the same keypair from the file | VERIFIED | `load_or_create` path.exists() branch at lib.rs:58-67; `test_load_after_create_is_identical` passes |
| 4 | A corrupt key file returns IdentityError::CorruptKeyFile | VERIFIED | `bytes.len() != 32` check at lib.rs:61-63; `test_corrupt_key_file_error` passes with `CorruptKeyFile(16)` |
| 5 | The SHA-256 fingerprint is deterministic (same seed -> same 64-char lowercase hex) | VERIFIED | SHA-256 of `verifying_key().to_bytes()` at lib.rs:133-136; `test_fingerprint_determinism` passes |
| 6 | periphored startup logs fingerprint at info level after identity load | VERIFIED | `tracing::info!(fingerprint = %identity.fingerprint_hex(), "identity loaded")` at main.rs:72-75 |
| 7 | GetStatus IPC response includes the real fingerprint_hex string | VERIFIED | `fingerprint: Some(identity.fingerprint_hex())` at main.rs:136; `get_status_returns_status_response` IPC test passes |
| 8 | identicon() returns an 11-line OpenSSH Drunken Bishop string with correct header/footer | VERIFIED | `drunken_bishop()` at lib.rs:150-202; `test_identicon_borders`, `test_identicon_line_count`, `test_identicon_determinism` all pass |
| 9 | word_phrase() returns exactly 6 lowercase BIP39 words, deterministic | VERIFIED | `word_indices()` + `BIP39_WORDS` indexing at lib.rs:127-131; `test_word_phrase_determinism`, `test_word_phrase_validity` pass |
| 10 | Identicon display can be disabled via config (show_identicon=false suppresses identicon in GetIdenticon response) | VERIFIED | `resolve_identicon(config.identity.show_identicon, &identity)` at main.rs:155; `test_show_identicon_suppressed_when_disabled` proves empty string returned when false; `test_show_identicon_returned_when_enabled` proves non-empty 11-line string when true. Both tests pass. |

**Score:** 10/10 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/periphore-identity/src/lib.rs` | IdentityStore, IdentityError, load_or_create, fingerprint_hex, identicon, word_phrase, default_key_path | VERIFIED | All exports present and substantive; drunken_bishop and word_indices private helpers implemented |
| `crates/periphore-identity/src/bip39.rs` | BIP39_WORDS static &[&str; 2048] with compile-time assertion | VERIFIED | 2048 words from "abandon" to "zoo"; `assert!(BIP39_WORDS.len() == 2048)` present |
| `crates/periphore-identity/tests/identity.rs` | 9 test functions, all active (0 ignored) | VERIFIED | 9/9 pass: 4 SEC-01 + 3 SEC-02 + 2 SEC-03 tests |
| `crates/periphore-protocol/src/ipc.rs` | IpcResponse::Identicon and IpcResponse::WordPhrase variants | VERIFIED | Both variants present with correct serde tag attributes |
| `crates/periphore-config/src/schema.rs` | IdentityConfig with show_identicon: bool field | VERIFIED | Struct present with Default impl (show_identicon: true); Config.identity field wired |
| `crates/periphore-config/src/lib.rs` | IdentityConfig re-exported in pub use | VERIFIED | `pub use schema::{..., IdentityConfig, ...}` present |
| `crates/periphored/src/main.rs` | Identity loaded at startup, GetIdenticon/WordPhrase dispatch wired, resolve_identicon helper gates show_identicon | VERIFIED | resolve_identicon at line 29; called with config.identity.show_identicon at line 155; two suppression tests pass |
| `crates/periphore-protocol/tests/roundtrip.rs` | Round-trip tests for Identicon and WordPhrase | VERIFIED | Both variants in `ipc_response_all_variants_round_trip`; test passes |
| `crates/periphore-config/tests/config.rs` | identity_show_identicon_defaults_to_true + _can_be_disabled_via_toml | VERIFIED | Both config parsing tests pass |
| `Cargo.toml` | rand_core workspace dep with getrandom feature | VERIFIED | Present in workspace dependencies |
| `crates/periphored/Cargo.toml` | tempfile dev-dependency for test key file creation | VERIFIED | `tempfile = "3"` in [dev-dependencies] |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/periphored/src/main.rs` | `crates/periphore-identity/src/lib.rs` | `IdentityStore::load_or_create(&key_path)` | WIRED | Line 70; `default_key_path()` at line 68 |
| `crates/periphore-identity/src/lib.rs` | key file on disk | `OpenOptionsExt::mode(0o600)` at `create_new` | WIRED | Lines 81-89; atomic 0600 creation confirmed |
| `crates/periphored/src/main.rs GetIdenticon arm` | `resolve_identicon()` | `config.identity.show_identicon` gate | WIRED | Line 155: `resolve_identicon(config.identity.show_identicon, &identity)` — gap from prior verification is closed |
| `resolve_identicon(false, ...)` | empty string | conditional return in free function | WIRED | Lines 29-35 in main.rs; proven by `test_show_identicon_suppressed_when_disabled` |
| `crates/periphored/src/main.rs select! arms` | `IpcResponse::Identicon / WordPhrase` | `resolve_identicon()` and `identity.word_phrase()` calls | WIRED | Lines 151-163 |
| `IdentityStore::identicon` | `self.fingerprint [u8; 32]` | `drunken_bishop(&self.fingerprint)` | WIRED | lib.rs:119 |
| `IdentityStore::word_phrase` | `BIP39_WORDS static` | `word_indices(&self.fingerprint)` | WIRED | lib.rs:127-131 |
| `crates/periphore-protocol/tests/roundtrip.rs` | `IpcResponse::Identicon + WordPhrase` | `ipc_resp_round_trip` helper | WIRED | Round-trip test passes |
| `config.identity.show_identicon` | GetIdenticon dispatch behavior | `resolve_identicon(config.identity.show_identicon, &identity)` | WIRED | Previously NOT WIRED — now fully wired; both unit tests prove behavioral correctness |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `periphored GetIdenticon arm` | `resolve_identicon(config.identity.show_identicon, &identity)` | `drunken_bishop(&self.fingerprint)` pure function over Ed25519 keypair; gated on config flag | Yes | FLOWING — and correctly gated |
| `periphored GetWordPhrase arm` | `identity.word_phrase()` | `word_indices(&self.fingerprint)` + `BIP39_WORDS` static | Yes | FLOWING |
| `periphored GetStatus arm` | `identity.fingerprint_hex()` | SHA-256 of loaded/generated keypair | Yes | FLOWING |
| `IdentityConfig::show_identicon` | conditional identicon suppression in GetIdenticon arm | `resolve_identicon(config.identity.show_identicon, &identity)` at main.rs:155 | Yes | FLOWING — previously DISCONNECTED, now wired |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace compiles | `cargo build --workspace` | Exit 0 | PASS |
| All 9 identity tests pass | `cargo test -p periphore-identity` | 9/9 pass, 0 ignored | PASS |
| All 7 config tests pass | `cargo test -p periphore-config` | 7/7 pass | PASS |
| All 4 protocol round-trip tests pass | `cargo test -p periphore-protocol` | 4/4 pass | PASS |
| All 8 IPC socket tests pass | `cargo test -p periphore-ipc` | 8/8 pass | PASS |
| show_identicon suppression tests | `cargo test -p periphored` | test_show_identicon_suppressed_when_disabled ok; test_show_identicon_returned_when_enabled ok | PASS |
| Workspace test total | `cargo test --workspace` | 32 tests, 0 failed, 0 ignored | PASS |
| resolve_identicon defined and called with show_identicon | `grep -n "resolve_identicon" crates/periphored/src/main.rs` | 5 matches: fn def (line 29), call site (line 155), use in test (lines 248, 267, 278) | PASS |
| config.identity.show_identicon referenced in dispatch | `grep -n "config.identity.show_identicon" crates/periphored/src/main.rs` | Line 155 match | PASS |
| Unconditional identity.identicon() call removed from dispatch | Line 31 occurrence is inside resolve_identicon body, not in select! dispatch arm | Dispatch arm uses resolve_identicon; no bare unconditional call in select! | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| SEC-01 | 02-01 | Ed25519 keypair generation and persistence | SATISFIED | `IdentityStore::load_or_create` with 0600 key file; 4 SEC-01 tests pass; fingerprint in GetStatus |
| SEC-02 | 02-02, 02-03 | Identicon (Drunken Bishop) for visual verification | SATISFIED | `drunken_bishop()` implemented; `test_identicon_borders` (exact header/footer), `test_identicon_line_count` (11 lines), `test_identicon_determinism` pass; GetIdenticon IPC arm returns `IpcResponse::Identicon` |
| SEC-03 | 02-02, 02-03 | Word phrase (6 BIP39 words) for typed verification | SATISFIED | `word_indices()` + BIP39_WORDS; `test_word_phrase_validity` (6 lowercase words), `test_word_phrase_determinism` pass; GetWordPhrase returns `IpcResponse::WordPhrase` |
| SEC-04 | 02-03, 02-04 | Identicon display can be disabled for headless setups | SATISFIED | `resolve_identicon(config.identity.show_identicon, &identity)` in GetIdenticon arm; `test_show_identicon_suppressed_when_disabled` proves empty string when false; two config parsing tests confirm TOML can set false |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/periphore-cli/src/lib.rs` | 16 | `anyhow::bail!("not yet implemented")` | Info | CLI stub — Phase 5 per ROADMAP; not blocking Phase 2 goal |
| `crates/periphored/src/main.rs` | 126 | SIGHUP handler is a placeholder | Info | Explicitly noted as Phase 4 concern; not blocking Phase 2 goal |

No blocker anti-patterns remain. The previously identified blocker (unconditional `identity.identicon()` in GetIdenticon dispatch) is resolved.

### Human Verification Required

#### 1. Cross-Platform Identicon Visual Identity Check

**Test:** Generate an Ed25519 keypair with a fixed 32-byte seed (e.g., `[0u8; 32]`) on both macOS and Linux. Run the identicon function on the resulting fingerprint on each platform. Compare the output character-by-character.
**Expected:** The identicon strings are identical on both platforms (both header, all 9 grid rows, footer). The Drunken Bishop algorithm is deterministic pure Rust with no platform-conditional branches.
**Why human:** ROADMAP SC3 explicitly requires verification on both macOS and Linux. This cannot be automated in a single-machine CI run. Risk is low given the implementation uses no platform-dependent code paths, but the ROADMAP contract requires explicit confirmation.

### Gaps Summary

No gaps remain. The single gap from the prior verification (show_identicon flag never read in GetIdenticon dispatch) has been fully closed by plan 02-04:

- `resolve_identicon(show_identicon: bool, identity: &IdentityStore) -> String` helper extracted above `fn main()`
- GetIdenticon arm now calls `resolve_identicon(config.identity.show_identicon, &identity)` at line 155
- `test_show_identicon_suppressed_when_disabled` proves empty string returned when `show_identicon=false`
- `test_show_identicon_returned_when_enabled` proves non-empty 11-line string returned when `show_identicon=true`
- `cargo test --workspace` exits 0 with 32 tests passing, 0 failed

All 10 observable truths are VERIFIED. All 4 requirements (SEC-01, SEC-02, SEC-03, SEC-04) are SATISFIED. The only remaining item is a cross-platform human check that ROADMAP SC3 calls out explicitly.

---

_Verified: 2026-04-22_
_Verifier: Claude (gsd-verifier)_
