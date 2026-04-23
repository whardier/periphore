---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
current_phase: 3
current_plan: 0
status: ready
stopped_at: "Phase 2 complete — all 4 plans done, 32 tests pass, SEC-01/02/03/04 all satisfied, cross-platform identicon verified"
last_updated: "2026-04-23T00:00:00Z"
progress:
  total_phases: 10
  completed_phases: 2
  total_plans: 10
  completed_plans: 10
  percent: 20
---

# Project State

**Project:** Periphore
**Milestone:** 1 -- v1 Core
**Current phase:** 3
**Current plan:** 0 (phase 3 not started)
**Status:** Ready
**Last updated:** 2026-04-23

---

## Project Reference

**Core value:** A machine's input devices should be able to reach any peer on the network, flowing naturally across screen edges, with verified identity and no central authority.

**Current focus:** Phase 03 -- Configuration & Trust Persistence (not yet started)

---

## Current Position

Phase: 02 (Identity & Cryptography) -- COMPLETE
Phase: 03 (Configuration & Trust Persistence) -- NEXT
**Phase:** 2 of 10 complete
**Progress:** [██░░░░░░░░] 20%

---

## Performance Metrics

| Metric | Value |
|--------|-------|
| Phases complete | 2/10 |
| Plans complete | 10/10 (phases 1+2) |
| Requirements delivered | SEC-01, SEC-02, SEC-03, SEC-04, CFG-01, IPC-01, IPC-02 (7/30) |
| Session count | 5 |

---

## Accumulated Context

### Key Decisions

- Cargo workspace architecture with 11 crates: periphore-protocol, periphore-config, periphore-identity, periphore-core, periphore-ipc, periphore-cli (library), periphore-net, periphore-capture, periphore-inject, periphore (CLI binary entry), periphored (daemon binary entry)
- Build order follows dependency chain: protocol -> config+identity -> core+ipc+ctl -> net -> capture+inject
- TCP-only transport for SSH tunnelability
- Captive window before seamless accessibility-based input
- Config never auto-writes; all config is user-authored
- Clippy pedantic group requires priority=-1 for individual lint overrides (Rust 1.95.0 lint_groups_priority)
- Cargo.lock committed since workspace produces binary crates
- All periphore-protocol tests in tests/roundtrip.rs (integration test) because [lib] test=false prevents inline test modules
- IpcRequest/IpcResponse use serde tag="type" with rename_all="snake_case" for JSON-lines IPC protocol
- Config defaults via #[serde(default)] + Default impls instead of Figment Serialized::defaults (avoids Serialize on Config, preserves CFG-01)
- ENV_MUTEX in config tests serializes access to PERIPHORE_* env vars for thread-safe parallel testing
- IpcCommand uses oneshot responder pattern: each variant carries oneshot::Sender<IpcResponse> for bidirectional IPC over Unix socket
- Each IPC integration test uses unique temp socket path with PID suffix for parallel-safe test isolation
- tokio::select! macro does not support #[cfg(unix)] on arms; guards placed on signal variable declarations instead
- periphored uses exhaustive send_ok() helper for IpcCommand dispatch with compiler-enforced variant coverage
- periphore-protocol added as direct dependency of periphored for IpcResponse type access
- periphore main.rs uses eprintln! for stub messages to keep stdout clean for future structured output
- periphore-cli uses anyhow (not thiserror) because its sole consumer is the periphore binary entry point
- rand_core 0.6 used directly (not rand 0.8/0.9) to avoid version conflict with ed25519-dalek 2.2 rand_core feature gate
- OpenOptionsExt::mode(0o600) with create_new(true) for atomic key file creation — eliminates world-readable race window
- Debug derive added to IdentityStore — required for Result<IdentityStore, IdentityError> in test panic messages
- resolve_identicon() extracted as pure free function in periphored/src/main.rs — testable without the async daemon
- tempfile dev-dep added to periphored for SEC-04 unit tests (not workspace-pinned — noted in REVIEW.md IN-02)
- Drunken Bishop output is character-for-character identical on macOS (darwin 25.4.0) and Linux (rust:1-slim) — ROADMAP SC3 verified

### Open TODOs

- CR-01 (Critical): periphored main loop spins at 100% CPU when IPC task exits cleanly — JoinSet::join_next() on empty set needs `if !tasks.is_empty()` guard + `break` on `Some(Ok(Ok(())))` arm
- WR-01 (Warning): IdentityStore::keypair is pub — exposes raw private key material; should be private with sign()/verifying_key() accessors
- WR-02 (Warning): build_border() panics on label > 13 chars — add assert!() to document invariant
- WR-03 (Warning): key file written without sync_all() — identity lost on power failure during first-run
- IN-03 (Info): missing key file 0600 permission test in identity test suite

### Blockers

(None)

---

## Session Continuity

### Last Session

- **Date:** 2026-04-23
- **Work done:** Phase 2 gap closure (plan 02-04) — SEC-04 fully satisfied; resolve_identicon() helper extracted and GetIdenticon gated on config.identity.show_identicon; 2 unit tests added; cross-platform identicon verified via Docker (macOS == Linux); VERIFICATION.md status: passed; ROADMAP phase 2 marked complete
- **Stopped at:** Phase 2 complete — ready for Phase 3
- **Next action:** /gsd:discuss-phase 3 or /gsd:plan-phase 3
