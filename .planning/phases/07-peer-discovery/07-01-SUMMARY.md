---
phase: 07-peer-discovery
plan: 01
subsystem: protocol
tags: [mdns, discovery, ipc, config, rust, serde]

# Dependency graph
requires:
  - phase: 06-tcp-peering
    provides: IpcRequest/IpcResponse/IpcCommand patterns that DiscoveredPeers extends
provides:
  - DiscoveredPeerInfo struct (hostname, port, last_seen_epoch, source) in periphore-protocol
  - GetDiscoveredPeers IpcRequest variant and DiscoveredPeers IpcResponse variant
  - DiscoveryConfig struct (enabled, instance_name, service_type, ssh_probe_enabled, ssh_probe_ports) in periphore-config
  - GetDiscoveredPeers IpcCommand variant with oneshot responder in periphore-ipc
  - mdns-sd and periphore-discovery workspace dependency declarations in Cargo.toml
affects: [07-02-PLAN, 07-03-PLAN, 07-04-PLAN]

# Tech tracking
tech-stack:
  added: [mdns-sd 0.19 (workspace dep declared)]
  patterns:
    - DiscoveryConfig uses serde(default) + manual Default impl returning enabled=false (opt-in per CFG-01)
    - DiscoveredPeerInfo uses last_seen_epoch (u64 seconds) for IPC serialization; avoids Instant
    - GetDiscoveredPeers follows exact oneshot responder pattern of all other IpcCommand variants

key-files:
  created: []
  modified:
    - crates/periphore-protocol/src/ipc.rs
    - crates/periphore-protocol/src/lib.rs
    - crates/periphore-protocol/tests/roundtrip.rs
    - crates/periphore-config/src/schema.rs
    - crates/periphore-ipc/src/lib.rs
    - Cargo.toml

key-decisions:
  - "DiscoveryConfig defaults to enabled=false (opt-in, CFG-01 compliant) — no automatic discovery on restricted networks"
  - "DiscoveredPeerInfo.last_seen_epoch is u64 Unix epoch seconds — avoids Instant serialization pitfall (Pitfall 6 in 07-RESEARCH.md)"
  - "mdns-sd added at workspace level (version 0.19); periphore-discovery dep pre-declared so Plan 02 crate can reference {workspace=true}"
  - "DiscoveredPeerInfo exported at periphore-protocol crate root alongside PendingPeerInfo"
  - "Roundtrip tests updated to cover new GetDiscoveredPeers and DiscoveredPeers variants"

patterns-established:
  - "Phase 7 type foundation pattern: protocol types first (Plan 01), then crate scaffold (Plan 02), then daemon wiring (Plan 03), then CLI (Plan 04)"

requirements-completed: [NET-02]

# Metrics
duration: 3min
completed: 2026-04-28
---

# Phase 07 Plan 01: Protocol Types and Config Schema for Discovery Summary

**DiscoveredPeerInfo, GetDiscoveredPeers IPC types, DiscoveryConfig schema, and mdns-sd workspace dep establish the Wave 0 interface contract for Phase 7 peer discovery**

## Performance

- **Duration:** 3 min
- **Started:** 2026-04-28T18:33:01Z
- **Completed:** 2026-04-28T18:36:17Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Added `GetDiscoveredPeers` to `IpcRequest`, `DiscoveredPeerInfo` struct, and `DiscoveredPeers` to `IpcResponse` in periphore-protocol
- Added `DiscoveryConfig` struct (5 fields, correct defaults) to periphore-config schema; wired as `Config.discovery` field
- Added `GetDiscoveredPeers` `IpcCommand` variant with oneshot responder pattern in periphore-ipc
- Declared `mdns-sd = { version = "0.19" }` and `periphore-discovery` as workspace dependencies in root Cargo.toml
- All 5 roundtrip tests pass including coverage of new `GetDiscoveredPeers` and `DiscoveredPeers` variants

## Task Commits

1. **Task 1: Protocol types and config schema for discovery** - `5b0b2ec` (feat)
2. **Task 2: IPC command wiring and workspace dependency declarations** - `73478ba` (feat)

## Files Created/Modified

- `crates/periphore-protocol/src/ipc.rs` - Added GetDiscoveredPeers variant, DiscoveredPeerInfo struct, DiscoveredPeers IpcResponse variant
- `crates/periphore-protocol/src/lib.rs` - Exported DiscoveredPeerInfo at crate root
- `crates/periphore-protocol/tests/roundtrip.rs` - Extended to cover new GetDiscoveredPeers and DiscoveredPeers variants
- `crates/periphore-config/src/schema.rs` - Added DiscoveryConfig struct (5 fields) and Config.discovery field
- `crates/periphore-ipc/src/lib.rs` - Added GetDiscoveredPeers IpcCommand variant with oneshot responder and match arm
- `Cargo.toml` - Added mdns-sd and periphore-discovery to [workspace.dependencies]

## Decisions Made

- `DiscoveryConfig` does not derive `Serialize` — enforces CFG-01 at compile time (no config auto-write paths possible)
- `last_seen_epoch: u64` chosen over `SystemTime` for IPC — `Instant` cannot be serialized; u64 epoch seconds are unambiguous
- `mdns-sd = { version = "0.19" }` pre-declared at workspace level so Plan 02 can use `{ workspace = true }` in crate Cargo.toml
- Roundtrip tests extended as a Rule 2 deviation (correctness: new types need coverage)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Extended roundtrip tests to cover new IPC types**
- **Found during:** Task 1 (Protocol types and config schema for discovery)
- **Issue:** New `GetDiscoveredPeers` (IpcRequest) and `DiscoveredPeers` (IpcResponse) variants added but not covered by existing `ipc_request_all_variants_round_trip` and `ipc_response_all_variants_round_trip` tests; also `DiscoveredPeerInfo` not imported in test file
- **Fix:** Added `DiscoveredPeerInfo` import, `IpcRequest::GetDiscoveredPeers` to request roundtrip cases, `IpcResponse::DiscoveredPeers` with sample data to response roundtrip cases; exported `DiscoveredPeerInfo` from lib.rs crate root
- **Files modified:** crates/periphore-protocol/tests/roundtrip.rs, crates/periphore-protocol/src/lib.rs
- **Verification:** `cargo test -p periphore-protocol` passes (5/5 tests)
- **Committed in:** 5b0b2ec (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 missing critical — test coverage)
**Impact on plan:** Auto-fix necessary for correctness; no scope creep.

## Issues Encountered

None — plan executed cleanly. The `crates/*` glob in workspace members handles the not-yet-created `periphore-discovery` crate correctly (glob only matches existing directories).

## Known Stubs

None — this plan adds type definitions only; no UI rendering, no data sources. The `GetDiscoveredPeers` IpcCommand variant reaches the `_ => {}` wildcard arm in `periphored/src/main.rs::send_ok`, which is the correct stub-free placeholder until Plan 03 wires the discovery service.

## Next Phase Readiness

- Plan 02 (`periphore-discovery` crate scaffold) can now reference `{ workspace = true }` for `mdns-sd` and `periphore-discovery` dep
- Plan 03 (daemon wiring) has all IPC types it needs to add `IpcCommand::GetDiscoveredPeers` dispatch arm
- Plan 04 (CLI) has `IpcRequest::GetDiscoveredPeers` and `IpcResponse::DiscoveredPeers` available for `periphore peers discovered` command

---
*Phase: 07-peer-discovery*
*Completed: 2026-04-28*
