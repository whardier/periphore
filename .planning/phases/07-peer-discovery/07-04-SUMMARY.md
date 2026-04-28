---
phase: 07-peer-discovery
plan: 04
subsystem: discovery
tags: [rust, tokio, mdns, ssh-probe, integration-tests, periphored, periphore-discovery]

# Dependency graph
requires:
  - phase: 07-peer-discovery
    plan: 01
    provides: IpcCommand::GetDiscoveredPeers, IpcResponse::DiscoveredPeers, DiscoveredPeerInfo types
  - phase: 07-peer-discovery
    plan: 02
    provides: DiscoveryService, DiscoveryEvent, DiscoveredPeerList, DiscoverySource crate
  - phase: 07-peer-discovery
    plan: 03
    provides: periphore peers discovered/pending CLI handlers (IPC consumers)

provides:
  - periphored daemon fully wired with DiscoveryService (start, discovered_list, cancel)
  - GetDiscoveredPeers IPC dispatch arm returning live discovered list snapshot
  - Discovery events (PeerDiscovered, PeerRemoved, Error) handled in daemon select! loop
  - Graceful discovery cancel via CancellationToken before tasks.abort_all()
  - 7 integration tests covering list cap, GC, goodbye removal, snapshot, upsert, SSH probe, self-detection

affects: [08-topology-routing, phase-09-capture]

# Tech tracking
tech-stack:
  added: [tokio-util (CancellationToken in periphored)]
  patterns:
    - DiscoveryService spawned conditional on config.discovery.enabled || config.discovery.ssh_probe_enabled
    - discovery_cancel.cancel() before tasks.abort_all() for graceful shutdown ordering
    - Integration tests for probe: real TcpListener responds with HelloAck; DiscoveryService public API tested
    - ssh_probe_against_test_listener uses looping accept() — probe may connect multiple times before event

key-files:
  created:
    - crates/periphore-discovery/tests/integration.rs
  modified:
    - crates/periphored/Cargo.toml
    - crates/periphored/src/main.rs
    - crates/periphore-ipc/tests/socket.rs

key-decisions:
  - "GC task always spawned by DiscoveryService.start() regardless of enabled/ssh_probe_enabled — daemon may call start() conditionally but GC is internal invariant; conditional guard in daemon uses || so GC runs when either feature is on"
  - "Integration test listener uses looping accept() in tokio::spawn — probe may connect once to 'discover' but the listener must be alive for the full test duration"
  - "GetDiscoveredPeers IPC fix in periphore-ipc socket test: was missing from handle_test_command exhaustive match — added IpcResponse::DiscoveredPeers { peers: vec![] } arm"

patterns-established:
  - "Discovery service wired after ConnectionManager and before IPC server spawn — consistent with Phase 6 ordering"
  - "Each integration test uses tokio::time::timeout() for bounded async test duration"
  - "Probe test uses looping accept + per-connection tokio::spawn — handles probe reconnect between sweeps"

requirements-completed: [NET-02]

# Metrics
duration: 5min
completed: 2026-04-28
---

# Phase 07 Plan 04: Daemon Discovery Wiring + Integration Tests Summary

**DiscoveryService wired into periphored select! loop with GetDiscoveredPeers IPC dispatch, graceful cancel, and 7 integration tests covering D-07/D-08/D-09 list behaviors and NET-02-SSH probe validation**

## Performance

- **Duration:** 5 min
- **Started:** 2026-04-28T18:50:17Z
- **Completed:** 2026-04-28T18:55:44Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Added `periphore-discovery` and `tokio-util` to `crates/periphored/Cargo.toml` workspace deps
- Wired `DiscoveryService::new()`, `start()`, and `discovered_list()` into `periphored/src/main.rs` with conditional spawn based on `config.discovery.enabled || config.discovery.ssh_probe_enabled`
- Added `discovery_event` arm in daemon `select!` loop handling `PeerDiscovered`, `PeerRemoved`, and `Error` events with appropriate `tracing::info!`/`tracing::warn!` calls
- Added `IpcCommand::GetDiscoveredPeers` dispatch arm calling `discovery_service.discovered_list()` and responding with `IpcResponse::DiscoveredPeers { peers }`
- Added `discovery_cancel.cancel()` before `tasks.abort_all()` for ordered graceful shutdown
- Created 7 integration tests in `crates/periphore-discovery/tests/integration.rs`: all pass
- Fixed non-exhaustive match in `periphore-ipc/tests/socket.rs` (Rule 1 auto-fix)
- Full workspace build and test suite green

## Task Commits

1. **Task 1: Daemon wiring -- DiscoveryService + select! loop + IPC dispatch** - `24cf23b` (feat)
2. **Task 2: Integration tests for periphore-discovery** - `347b75f` (feat)

## Files Created/Modified

- `crates/periphored/Cargo.toml` - Added periphore-discovery and tokio-util workspace deps
- `crates/periphored/src/main.rs` - DiscoveryService init, discovery_event select! arm, GetDiscoveredPeers IPC dispatch, graceful cancel
- `crates/periphore-discovery/tests/integration.rs` - 7 integration tests: list_cap_eviction, gc_removes_expired, remove_by_fullname, snapshot_converts_instant_to_epoch, upsert_refreshes_last_seen, ssh_probe_against_test_listener, ssh_probe_skips_own_fingerprint
- `crates/periphore-ipc/tests/socket.rs` - Added GetDiscoveredPeers arm + corrected GetPendingVerifications response (Rule 1 auto-fix)

## Decisions Made

- Integration test listener for SSH probe test uses looping `accept()` with per-connection `tokio::spawn` — the probe may reconnect between test checks; a single-accept listener would hang if the probe retries
- `DiscoveryConfig` is constructed directly in tests using named struct literals — fields are pub, no builder pattern required
- Self-detection test (Test 7) expects `Timeout` as the success path — no `PeerDiscovered` arriving within 2 seconds confirms the self-skip logic works

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed non-exhaustive IpcCommand match in periphore-ipc socket test**
- **Found during:** Task 2 (cargo test --workspace after adding integration tests)
- **Issue:** `crates/periphore-ipc/tests/socket.rs::handle_test_command` was missing `IpcCommand::GetDiscoveredPeers` arm (added to IpcCommand in Plan 01). Also `GetPendingVerifications` was returning `IpcResponse::Ok` instead of `IpcResponse::PendingPeers { peers }` (wrong stub from before that variant existed).
- **Fix:** Added `IpcCommand::GetDiscoveredPeers { responder }` arm returning `IpcResponse::DiscoveredPeers { peers: vec![] }`; corrected `GetPendingVerifications` to return `IpcResponse::PendingPeers { peers: vec![] }`
- **Files modified:** crates/periphore-ipc/tests/socket.rs
- **Verification:** `cargo test --workspace` passes (all test suites green)
- **Committed in:** 347b75f (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug — non-exhaustive match + incorrect stub response in IPC socket test)
**Impact on plan:** Required fix for compilation correctness. No scope creep.

## Issues Encountered

None beyond the auto-fixed deviation above.

## Known Stubs

None — periphored daemon fully wires DiscoveryService. The CLI (`periphore peers discovered`) sends GetDiscoveredPeers which the daemon now handles with the live discovered list. Integration tests cover all 7 required behaviors.

## Threat Surface Scan

No new network endpoints or trust boundaries introduced beyond what the plan's threat model covers:
- T-7-11 (DoS via discovery channel): mitigated — channel buffer is 64 (bounded); daemon select! arm processes events at main-loop cadence
- T-7-10 (Spoofing): accepted — events come from internal mpsc channel, not external network
- T-7-12 (Repudiation in test listener): accepted — test-only code with deterministic seeds

## Next Phase Readiness

- Phase 7 (Peer Discovery) is now complete — all 4 plans executed, NET-02 delivered
- `periphore peers discovered` end-to-end flow is live: CLI → IPC → daemon → DiscoveryService → DiscoveredPeerList snapshot
- Phase 8 (Topology Routing) can rely on the discovered peer list for routing decisions

## Self-Check: PASSED

- FOUND: crates/periphore-discovery/tests/integration.rs
- FOUND: crates/periphored/Cargo.toml (contains periphore-discovery)
- FOUND: crates/periphored/src/main.rs (contains DiscoveryService, GetDiscoveredPeers, discovery_cancel)
- FOUND: crates/periphore-ipc/tests/socket.rs (contains GetDiscoveredPeers arm)
- FOUND commit: 24cf23b (Task 1)
- FOUND commit: 347b75f (Task 2)

---
*Phase: 07-peer-discovery*
*Completed: 2026-04-28*
