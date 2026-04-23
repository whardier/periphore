---
phase: 01-workspace-protocol-foundation
plan: 02
subsystem: protocol
tags: [rust, serde, postcard, serde_json, wire-protocol, ipc, types]

# Dependency graph
requires:
  - phase: 01-workspace-protocol-foundation
    provides: Cargo workspace with periphore-protocol crate stub and dependencies
provides:
  - "PeerMessage enum with 16 wire protocol variants (postcard-serializable)"
  - "IpcRequest enum with 12 IPC command variants (serde_json-serializable)"
  - "IpcResponse enum with 4 response variants (serde_json-serializable)"
  - "Supporting types: MonitorInfo, Edge, EdgeMapping, InputEvent, MouseEventData, KeyEventData"
  - "Re-export facade at crate root for clean import paths"
  - "Round-trip tests for all variants (postcard + serde_json)"
affects: [01-04, 01-05, 01-06, phase-02, phase-04, phase-06]

# Tech tracking
tech-stack:
  added: []
  patterns: [postcard-round-trip-test, serde-json-tagged-enum, integration-tests-for-lib-test-false-crates]

key-files:
  created:
    - crates/periphore-protocol/src/types.rs
    - crates/periphore-protocol/src/peer.rs
    - crates/periphore-protocol/src/ipc.rs
    - crates/periphore-protocol/tests/roundtrip.rs
  modified:
    - crates/periphore-protocol/src/lib.rs
    - crates/periphore-protocol/Cargo.toml

key-decisions:
  - "All tests in tests/roundtrip.rs (integration test) because [lib] test=false prevents inline #[cfg(test)] modules from running"
  - "IpcRequest/IpcResponse use serde tag=type with rename_all=snake_case for JSON-lines IPC protocol"
  - "PeerMessage uses plain serde derive (no tag attribute) for postcard binary wire format"

patterns-established:
  - "Integration test pattern for crates with [lib] test=false: tests/ directory with external crate imports"
  - "postcard round-trip test: to_allocvec -> from_bytes -> assert_eq for every enum variant"
  - "serde_json round-trip test: to_string -> from_str -> serialize both and compare JSON strings"
  - "Tagged JSON enum: #[serde(rename_all = snake_case, tag = type)] for IPC request/response enums"

requirements-completed: [CFG-01, IPC-01, IPC-02]

# Metrics
duration: 4min
completed: 2026-04-23
---

# Phase 1 Plan 02: Protocol Types Summary

**Full wire protocol (PeerMessage, 16 variants) and IPC message types (IpcRequest 12 variants, IpcResponse 4 variants) with postcard and serde_json round-trip tests**

## Performance

- **Duration:** 4 min
- **Started:** 2026-04-23T02:03:44Z
- **Completed:** 2026-04-23T02:07:53Z
- **Tasks:** 2
- **Files created:** 4
- **Files modified:** 2

## Accomplishments
- Complete PeerMessage enum with all 16 wire protocol variants covering handshake, topology, focus token, input events, and control messages
- Complete IpcRequest enum with all 12 IPC command variants including InjectInputEvent and SimulateEdgeCross testing primitives (D-19)
- Complete IpcResponse enum with 4 response variants (Status, Peers, Ok, Error)
- Supporting type surface: MonitorInfo, Edge, EdgeMapping, InputEvent, MouseEventData, KeyEventData
- All types re-exported at crate root for clean consumer imports
- 4 integration tests covering postcard and serde_json round-trips for every variant

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement types.rs and peer.rs** - `3a8a1a3` (feat)
2. **Task 2: Implement ipc.rs, lib.rs re-exports, and roundtrip tests** - `e6818cb` (feat)

## Files Created/Modified
- `crates/periphore-protocol/src/types.rs` - MonitorInfo, Edge, EdgeMapping, InputEvent, MouseEventData, KeyEventData
- `crates/periphore-protocol/src/peer.rs` - PeerMessage enum with 16 variants (postcard-serializable)
- `crates/periphore-protocol/src/ipc.rs` - IpcRequest (12 variants) and IpcResponse (4 variants) with serde_json
- `crates/periphore-protocol/tests/roundtrip.rs` - Round-trip tests for all PeerMessage, IpcRequest, and IpcResponse variants
- `crates/periphore-protocol/src/lib.rs` - Re-export facade: pub mod ipc/peer/types + pub use for all public types
- `crates/periphore-protocol/Cargo.toml` - Added postcard and serde_json to dev-dependencies for integration tests

## Decisions Made
- All tests placed in tests/roundtrip.rs (integration test style) because the crate has `[lib] test = false` per D-07, which prevents inline `#[cfg(test)]` modules from executing under `cargo test`
- IpcRequest/IpcResponse use `#[serde(rename_all = "snake_case", tag = "type")]` for human-readable JSON-lines IPC protocol per D-16
- PeerMessage uses plain serde derive without tag attributes since postcard uses compact binary format, not JSON
- Round-trip test for IpcRequest compares serialized JSON strings rather than direct PartialEq (tagged enums with inner types are most reliably compared via their serialized form)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Known Stubs
- `PeerMessage::Hello.fingerprint` field documented as `[0u8; 32]` placeholder until Phase 2 identity implementation -- this is by design per D-11 and will be addressed in Phase 2

## TDD Gate Compliance
- RED: roundtrip.rs written first, failed to compile (IpcRequest/IpcResponse not yet defined) -- confirmed failing
- GREEN: ipc.rs + lib.rs re-exports implemented, all 4 tests pass -- confirmed passing
- REFACTOR: not needed -- code is clean as written

## Next Phase Readiness
- Protocol type surface complete; all subsequent crates can import from periphore-protocol
- Ready for Plan 03 (config schema) and Plan 04 (IPC implementation) which depend on these types
- No blockers or concerns

---
*Phase: 01-workspace-protocol-foundation*
*Completed: 2026-04-23*
