---
phase: "06"
plan: "06-05"
subsystem: tcp-peering
tags: [periphore-net, periphored, integration-tests, handshake, net-wiring, NET-01, NET-03]
dependency_graph:
  requires:
    - "06-01 (periphore-net Cargo.toml, DaemonConfig.listen, PendingPeerInfo)"
    - "06-02 (NetError, codec, PeerEvent, connection types)"
    - "06-03 (handshake.rs, manager.rs, lib.rs)"
    - "06-04 (periphored wiring, ConnectionManager in daemon)"
  provides:
    - NET-01 handshake integration tests (trusted, pending, version mismatch, fingerprint conflict, promote_pending, codec roundtrip)
    - NET-03 auto-connect integration test (PeerConfig.host triggers spawn_connector)
    - D-03 GetPendingVerifications IPC dispatch test
    - pub mod handshake and pub mod connection in periphore-net (enables integration test access)
  affects:
    - crates/periphore-net/tests/integration.rs
    - crates/periphore-net/src/lib.rs
    - crates/periphored/tests/net_wiring.rs
tech_stack:
  added:
    - pub mod handshake (was private mod — made public to allow integration test access)
    - pub mod connection (was private mod — made public for HandshakeResult access)
  patterns:
    - TcpListener::bind("127.0.0.1:0") for OS-assigned test ports (no hardcoded ports)
    - tokio::time::timeout around all async assertions (no CI hang risk)
    - Arc<IdentityStore> + Arc<RwLock<TrustStore>> for multi-task identity sharing
    - oneshot channel pairs for in-process IPC dispatch simulation
    - run_handshake_pair helper abstracts listener setup and parallel task execution
key_files:
  created:
    - crates/periphore-net/tests/integration.rs
    - crates/periphored/tests/net_wiring.rs
  modified:
    - crates/periphore-net/src/lib.rs
key_decisions:
  - "pub mod handshake and pub mod connection added to lib.rs — integration tests require direct access to perform_handshake_initiator and perform_handshake_responder; without pub the test file cannot import them (Rule 2 auto-fix)"
  - "run_handshake_pair_with_configs variant added — fingerprint_conflict test needs per-side PeerConfig injection; base run_handshake_pair delegates to it with None/None defaults"
  - "promote_pending test uses pre-bind TOCTOU pattern (pre-bind→drop→spawn_listener rebinds) — acceptable for in-process tests; flakiness not observed in 6 runs"

requirements-completed:
  - NET-01
  - NET-03

duration: 3min
completed: "2026-04-27"
---

# Phase 6 Plan 5: Integration tests — periphore-net handshake + periphored net wiring Summary

**6 NET-01 handshake tests (trusted peer, unknown peer, version mismatch, fingerprint conflict, codec roundtrip, promote_pending flow) and 2 NET-03/D-03 net wiring tests — all green under cargo test --workspace (91 tests, 0 failures). pub mod handshake and pub mod connection added to lib.rs to enable integration test access.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-04-27T09:20:59Z
- **Completed:** 2026-04-27T09:24:20Z
- **Tasks:** 2
- **Files modified:** 3 (2 created, 1 modified)

## Accomplishments

- Created `crates/periphore-net/tests/integration.rs` with 6 tests covering NET-01:
  - `handshake_trusted_peer` — both sides have each other's fingerprints; both return Trusted
  - `handshake_unknown_peer_goes_pending` — empty trust stores; both return Pending with correct peer fingerprint
  - `protocol_version_mismatch` — bad initiator sends version=99; responder returns Err(ProtocolVersion { got: 99 })
  - `codec_roundtrip_hello` — encode + decode PeerMessage::Hello is identity (non-async)
  - `fingerprint_conflict` — initiator has wrong configured fingerprint; returns Err(FingerprintConflict)
  - `promote_pending` — ConnectionManager pending→PeerPending event→promote_pending()→PeerConnected event
- Created `crates/periphored/tests/net_wiring.rs` with 2 tests covering NET-03 and D-03:
  - `peer_config_with_host_triggers_connector` — PeerConfig.host causes spawn_connector; PeerPending arrives
  - `pending_verifications_ipc` — GetPendingVerifications dispatch produces PendingPeers with empty list
- Made `pub mod handshake` and `pub mod connection` in `periphore-net/src/lib.rs` (previously private — integration tests require direct access)
- Introduced `run_handshake_pair` and `run_handshake_pair_with_configs` helpers for clean test setup
- All tests use `TcpListener::bind("127.0.0.1:0")` — no hardcoded ports
- All async assertions wrapped in `tokio::time::timeout` — no CI hang risk
- `cargo test --workspace` exits 0: 91 tests, 0 failures

## Task Commits

1. **Task 1: periphore-net integration tests (NET-01)** -- `ede0c96` (test)
2. **Task 2: periphored net_wiring tests (NET-03 + D-03)** -- `5b842c9` (test)

## Files Created/Modified

- `crates/periphore-net/tests/integration.rs` — 6 handshake integration tests (NET-01)
- `crates/periphored/tests/net_wiring.rs` — 2 net wiring integration tests (NET-03, D-03)
- `crates/periphore-net/src/lib.rs` — `pub mod handshake` and `pub mod connection` (was private)

## Decisions Made

- `pub mod handshake` / `pub mod connection` added as a Rule 2 (missing critical functionality) fix. The integration tests call `periphore_net::handshake::perform_handshake_initiator` and `periphore_net::handshake::perform_handshake_responder` directly — these are in the `handshake` module which was declared `mod handshake` (private) in `lib.rs`. Without the visibility change the test file fails to compile. The PLAN.md test code explicitly references these imports so the change aligns with the plan's intent.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical Functionality] Made pub mod handshake and pub mod connection in lib.rs**
- **Found during:** Task 1 (compile)
- **Issue:** `lib.rs` declared both `handshake` and `connection` as private modules. The integration test imports `periphore_net::handshake::perform_handshake_initiator`, `perform_handshake_responder`, and `periphore_net::connection::HandshakeResult` (via path). Private modules are not accessible from external integration tests.
- **Fix:** Changed `mod handshake;` to `pub mod handshake;` and `mod connection;` to `pub mod connection;` in `lib.rs`.
- **Files modified:** `crates/periphore-net/src/lib.rs`
- **Committed in:** `ede0c96` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (missing critical functionality)
**Impact on plan:** Minor visibility-only change. No behavioral change to existing code. All 20 pre-existing tests continue to pass.

## Known Stubs

None — all test assertions verify real behavior against real in-process connections.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. These are test files only — they run in the test binary, not in the daemon. No threat flags.

## Self-Check: PASSED

- `crates/periphore-net/tests/integration.rs`: EXISTS
- `crates/periphored/tests/net_wiring.rs`: EXISTS
- `grep "handshake_trusted_peer" crates/periphore-net/tests/integration.rs`: FOUND
- `grep "handshake_unknown_peer_goes_pending" crates/periphore-net/tests/integration.rs`: FOUND
- `grep "protocol_version_mismatch" crates/periphore-net/tests/integration.rs`: FOUND
- `grep "codec_roundtrip_hello" crates/periphore-net/tests/integration.rs`: FOUND
- `grep "fn fingerprint_conflict" crates/periphore-net/tests/integration.rs`: FOUND
- `grep "fn promote_pending" crates/periphore-net/tests/integration.rs`: FOUND
- `grep "FingerprintConflict" crates/periphore-net/tests/integration.rs`: FOUND
- `grep "PeerEvent::PeerConnected" crates/periphore-net/tests/integration.rs`: FOUND
- `grep "peer_config_with_host_triggers_connector" crates/periphored/tests/net_wiring.rs`: FOUND
- `grep "pending_verifications_ipc" crates/periphored/tests/net_wiring.rs`: FOUND
- `grep "PeerEvent::PeerPending" crates/periphored/tests/net_wiring.rs`: FOUND
- `grep "IpcResponse::PendingPeers" crates/periphored/tests/net_wiring.rs`: FOUND
- Commit `ede0c96`: FOUND
- Commit `5b842c9`: FOUND
- `cargo test -p periphore-net --test integration`: 6 tests, 0 failures
- `cargo test -p periphored --test net_wiring`: 2 tests, 0 failures
- `cargo test --workspace`: 91 tests, 0 failures
