---
plan: 02-04
phase: 02-identity-cryptography
status: complete
completed: 2026-04-22
tasks_total: 2
tasks_completed: 2
requirements_delivered:
  - SEC-04
key-files:
  created: []
  modified:
    - crates/periphored/src/main.rs
    - crates/periphored/Cargo.toml
self_check: PASSED
---

# Plan 02-04: SEC-04 Gap Closure — show_identicon Config Gating

## What Was Built

Closed the single remaining Phase 2 gap: `config.identity.show_identicon` is now consulted
in the daemon's `GetIdenticon` IPC dispatch arm. Previously the flag was correctly defined in
`IdentityConfig` and parsed from TOML, but never read — the identicon was always returned
unconditionally. This violated ROADMAP SC5 and SEC-04.

### Changes Made

**`crates/periphored/src/main.rs`** — Two changes:

1. Added `resolve_identicon(show_identicon: bool, identity: &IdentityStore) -> String` free
   function immediately before `fn main()`. Pure function with no async/side effects, designed
   to be unit-testable in isolation from the daemon event loop.

2. Replaced unconditional `identity.identicon()` call in the `GetIdenticon` select! arm with
   `resolve_identicon(config.identity.show_identicon, &identity)`. The `fingerprint_hex` field
   is still always returned (public information — not sensitive).

3. Added `#[cfg(test)] mod tests` with two SEC-04 unit tests:
   - `test_show_identicon_suppressed_when_disabled` — asserts empty string when `show_identicon=false`
   - `test_show_identicon_returned_when_enabled` — asserts 11-line Drunken Bishop string when `true`

**`crates/periphored/Cargo.toml`** — Added `[dev-dependencies]` section with `tempfile = "3"`
for deterministic temp key file creation in tests (mirrors the pattern in periphore-identity tests).

## Verification

All acceptance criteria met:

| Criterion | Result |
|-----------|--------|
| `resolve_identicon` has 2+ matches (fn def + call site) | PASS — lines 29, 155 |
| `config.identity.show_identicon` referenced in GetIdenticon arm | PASS — line 155 |
| Unconditional `identity.identicon()` call removed from dispatch | PASS — no match in select! arm |
| `cargo build -p periphored` exits 0 | PASS |
| `tempfile` in [dev-dependencies] | PASS |
| Both new SEC-04 tests present and passing | PASS |
| `cargo test --workspace` exits 0 | PASS — 32 tests, 0 failed |

## Requirements Delivered

| ID | Description | Evidence |
|----|-------------|----------|
| SEC-04 | Identicon display can be disabled for headless/automated setups | `resolve_identicon(false, …)` returns `""`, proven by `test_show_identicon_suppressed_when_disabled` |

## Deviations

None. Implementation follows the plan exactly.

## Self-Check: PASSED
