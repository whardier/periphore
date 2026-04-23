---
phase: 02-identity-cryptography
plan: 01
subsystem: identity
tags: [ed25519, sha2, rand_core, thiserror, directories, cryptography, keypair, fingerprint]

# Dependency graph
requires:
  - phase: 01-workspace-protocol-foundation
    provides: "Cargo workspace, periphore-ipc, periphore-protocol, periphored skeleton with IpcCommand dispatch"

provides:
  - "IdentityStore struct with load_or_create(), fingerprint_hex(), identicon() stub, word_phrase() stub"
  - "IdentityError enum (CorruptKeyFile, Io, NoDataDir) with thiserror derive"
  - "default_key_path() free function using ProjectDirs for XDG path resolution"
  - "periphore-identity/tests/identity.rs with 4 active SEC-01 tests + 5 ignored stubs for plan 02-02"
  - "periphored loads identity at startup; GetStatus fingerprint field populated with real Ed25519 fingerprint"

affects:
  - 02-02-identity-cryptography
  - 02-03-identity-cryptography
  - 06-tcp-peering

# Tech tracking
tech-stack:
  added:
    - "rand_core 0.6 (features: getrandom) — OsRng CSPRNG for Ed25519 keypair generation"
    - "ed25519-dalek 2.2 rand_core feature — SigningKey::generate via OsRng"
    - "thiserror 2.0 — IdentityError derive (already in workspace, first use in identity crate)"
    - "directories 6.0 — ProjectDirs for XDG data path resolution (already in workspace)"
    - "tempfile 3 — dev-dependency for identity integration tests"
  patterns:
    - "OpenOptionsExt::mode(0o600) at file creation — atomic key file with no world-readable race window"
    - "load_or_create() pattern: path.exists() branches into load vs generate"
    - "SHA-256 fingerprint of public key bytes as canonical node identity"
    - "thiserror #[from] io::Error for transparent I/O error wrapping in library crates"
    - "default_key_path() free function keeps path resolution out of daemon main.rs"

key-files:
  created:
    - "crates/periphore-identity/src/lib.rs — IdentityStore, IdentityError, default_key_path"
    - "crates/periphore-identity/src/bip39.rs — empty stub module (populated in plan 02-02)"
    - "crates/periphore-identity/tests/identity.rs — 9 test functions (4 active SEC-01, 5 ignored stubs)"
  modified:
    - "Cargo.toml — rand_core workspace dep added; ed25519-dalek rand_core feature enabled"
    - "crates/periphore-identity/Cargo.toml — rand_core, thiserror, directories, tracing deps added; tempfile dev-dep"
    - "crates/periphored/Cargo.toml — periphore-identity workspace dep added"
    - "crates/periphored/src/main.rs — identity loaded at startup; GetStatus fingerprint filled in"

key-decisions:
  - "rand_core 0.6 + getrandom feature used directly (not rand 0.8/0.9) — minimal dep, avoids rand_core version conflict with ed25519-dalek 2.2"
  - "OpenOptionsExt::mode(0o600) at create_new — eliminates world-readable race window (preferred over post-write set_permissions)"
  - "Debug derive added to IdentityStore — required for Result<IdentityStore, IdentityError> in test panic messages"
  - "identicon() and word_phrase() return empty stubs — intentional, plan 02-02 implements SEC-02/SEC-03"
  - "ed25519-dalek rand_core feature enabled at workspace level — propagates to all crates automatically"

patterns-established:
  - "Identity integration test file at crates/periphore-identity/tests/identity.rs — first integration test file in workspace following [lib] test=false pattern from Phase 1 D-07"
  - "Wave 0 stub pattern: ignored tests decorated with #[ignore = \"implemented in plan 02-02\"] for future-plan placeholders"

requirements-completed:
  - SEC-01

# Metrics
duration: 4min
completed: 2026-04-23
---

# Phase 2 Plan 01: Identity Foundation Summary

**Ed25519 keypair lifecycle with atomic 0600 key file creation, SHA-256 fingerprint derivation, and periphored GetStatus integration via IdentityStore::load_or_create()**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-04-23T04:29:51Z
- **Completed:** 2026-04-23T04:33:42Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments

- `IdentityStore::load_or_create()` generates or loads the Ed25519 keypair from the XDG data path, writes the 32-byte seed with mode 0600 using `OpenOptionsExt::create_new(true).mode(0o600)` (no world-readable race window)
- SHA-256 fingerprint of public key bytes exposed as 64-char lowercase hex via `fingerprint_hex()`; deterministic and cross-platform
- `periphored` loads identity at startup after config; `GetStatus` IPC response now returns the real fingerprint instead of `None`
- All 4 SEC-01 tests pass; 5 SEC-02/SEC-03 stub tests ignored pending plan 02-02

## Task Commits

Each task was committed atomically:

1. **Task 1: Wave 0 test scaffold** - `71fd1cb` (test)
2. **Task 2: IdentityStore implementation** - `b98f51e` (feat)
3. **Task 3: periphored startup wiring** - `54004c4` (feat)

**Plan metadata:** (docs commit follows)

_Note: Task 1 is the TDD RED phase; Task 2 is the TDD GREEN phase._

## Files Created/Modified

- `crates/periphore-identity/src/lib.rs` — Full IdentityStore implementation: load_or_create, fingerprint_hex, identicon stub, word_phrase stub, IdentityError enum, default_key_path free function
- `crates/periphore-identity/src/bip39.rs` — Empty stub module (populated in plan 02-02)
- `crates/periphore-identity/tests/identity.rs` — 4 active SEC-01 tests + 5 ignored SEC-02/SEC-03 stubs
- `Cargo.toml` — Added rand_core workspace dep; enabled rand_core feature on ed25519-dalek
- `crates/periphore-identity/Cargo.toml` — Added rand_core, thiserror, directories, tracing deps; tempfile dev-dep
- `crates/periphored/Cargo.toml` — Added periphore-identity workspace dep
- `crates/periphored/src/main.rs` — Identity loaded at startup; GetStatus fingerprint populated

## Decisions Made

- Used `rand_core 0.6` directly (not `rand 0.8/0.9`) to avoid version conflicts with ed25519-dalek 2.2's optional `rand_core ^0.6.4` feature gate
- Used `OpenOptionsExt::mode(0o600)` with `create_new(true)` for atomic key file creation — eliminates the world-readable race window that exists between `fs::write()` and `fs::set_permissions()`
- Added `#[derive(Debug)]` to `IdentityStore` (deviation — see below) to satisfy `{:?}` format in test panic message

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added `#[derive(Debug)]` to `IdentityStore`**
- **Found during:** Task 2 (GREEN phase — running `cargo test -p periphore-identity`)
- **Issue:** Test `test_corrupt_key_file_error` uses `{other:?}` in the panic message for `Result<IdentityStore, IdentityError>`, which requires `IdentityStore: Debug`. The plan's code sample omitted the derive.
- **Fix:** Added `#[derive(Debug)]` above `pub struct IdentityStore`. `SigningKey` from ed25519-dalek already implements `Debug`.
- **Files modified:** `crates/periphore-identity/src/lib.rs`
- **Verification:** `cargo test -p periphore-identity` passed with all 4 tests green
- **Committed in:** `b98f51e` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 2 — missing critical for test correctness)
**Impact on plan:** Minimal. `Debug` is standard for public types; the plan's sample simply omitted the derive. No scope changes.

## Known Stubs

| Stub | File | Line | Reason |
|------|------|------|--------|
| `identicon()` returns `String::new()` | `crates/periphore-identity/src/lib.rs` | 119-120 | SEC-02 (Drunken Bishop) implemented in plan 02-02 |
| `word_phrase()` returns `Vec::new()` | `crates/periphore-identity/src/lib.rs` | 128-129 | SEC-03 (BIP39) implemented in plan 02-02 |

These stubs do not block this plan's goal (SEC-01). They are intentional placeholders per the plan design.

## Issues Encountered

None beyond the `Debug` derive auto-fix documented above.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `periphore-identity` crate is fully functional for SEC-01: keypair generation, persistence, fingerprint derivation
- Plan 02-02 can implement `identicon()` and `word_phrase()` without touching the persistence layer
- Plan 02-03 can add `IpcResponse::Identicon` and `IpcResponse::WordPhrase` variants and wire full IPC dispatch
- `cargo test --workspace` is green: all prior Phase 1 tests unaffected

## Self-Check: PASSED

| Item | Status |
|------|--------|
| `crates/periphore-identity/src/lib.rs` | FOUND |
| `crates/periphore-identity/src/bip39.rs` | FOUND |
| `crates/periphore-identity/tests/identity.rs` | FOUND |
| `02-01-SUMMARY.md` | FOUND |
| Commit 71fd1cb (Task 1) | FOUND |
| Commit b98f51e (Task 2) | FOUND |
| Commit 54004c4 (Task 3) | FOUND |
| `pub struct IdentityStore` exported | FOUND |
| `pub enum IdentityError` exported | FOUND |
| `pub fn default_key_path` exported | FOUND |
| `rand_core` workspace dep with getrandom | FOUND |
| `thiserror`, `directories` deps in identity crate | FOUND |
| `identity.fingerprint_hex()` in GetStatus arm | FOUND |
| `periphore-identity` dep in periphored | FOUND |

---
*Phase: 02-identity-cryptography*
*Completed: 2026-04-23*
