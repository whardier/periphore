---
phase: "06"
plan: "06-01"
subsystem: "protocol + config + net-crate foundations"
tags: [protocol, config, cargo, tcp-peering, foundation]
dependency_graph:
  requires: []
  provides:
    - IpcResponse::PendingPeers variant
    - PendingPeerInfo struct
    - DaemonConfig.listen field
    - periphore-net full dep list
  affects:
    - crates/periphore-protocol/src/ipc.rs
    - crates/periphore-config/src/schema.rs
    - crates/periphore-net/Cargo.toml
    - Cargo.toml
tech_stack:
  added:
    - futures-util 0.3 (workspace dep)
    - periphore-identity (periphore-net dep)
    - periphore-trust (periphore-net dep)
    - periphore-config (periphore-net dep)
    - periphore-core (periphore-net dep)
    - postcard (explicit dep in periphore-net)
    - tempfile (periphore-net dev-dep)
  patterns:
    - Manual Default impl for structs with non-false bool defaults
    - serde default function pattern for #[serde(default = "fn")]
    - TDD red/green/refactor per-task cycle
key_files:
  created: []
  modified:
    - crates/periphore-protocol/src/ipc.rs
    - crates/periphore-protocol/src/lib.rs
    - crates/periphore-protocol/tests/roundtrip.rs
    - crates/periphore-config/src/schema.rs
    - crates/periphore-config/tests/config.rs
    - crates/periphore-net/Cargo.toml
    - Cargo.toml
    - Cargo.lock
decisions:
  - "PendingPeerInfo defined in ipc.rs adjacent to IpcResponse (not a separate module) per plan direction"
  - "DaemonConfig.Default derive replaced with manual impl to return listen=true (bool zero-value is false)"
  - "futures-util 0.3 added as workspace dep (needed by periphore-net for SinkExt/StreamExt)"
metrics:
  duration_minutes: 15
  completed_date: "2026-04-27"
  tasks_completed: 2
  files_changed: 8
---

# Phase 6 Plan 1: Protocol/config/net-crate foundations Summary

**One-liner:** PendingPeerInfo + IpcResponse::PendingPeers for GetPendingVerifications IPC, daemon.listen=true default in DaemonConfig, and periphore-net Cargo.toml extended with all 13 Phase 6 dependencies.

## What Was Built

This plan laid the compile-time foundations required by all subsequent Phase 6 plans (02-05):

1. **IpcResponse::PendingPeers variant** (`crates/periphore-protocol/src/ipc.rs`): New `PendingPeerInfo` struct with `fingerprint`, `identicon`, `word_phrase` fields. `IpcResponse::PendingPeers { peers: Vec<PendingPeerInfo> }` variant added after the existing `Peers` variant. Serializes as `{"type":"pending_peers","peers":[...]}` via the existing `#[serde(rename_all = "snake_case", tag = "type")]` attribute. Re-exported from crate root as `periphore_protocol::PendingPeerInfo`.

2. **DaemonConfig.listen field** (`crates/periphore-config/src/schema.rs`): `listen: bool` field with `#[serde(default = "default_listen")]` and `fn default_listen() -> bool { true }`. Replaced `#[derive(Default)]` with a manual `impl Default for DaemonConfig` that returns `listen: true` (bool default via derive would produce `false`, which is wrong for a P2P daemon).

3. **periphore-net Cargo.toml** (`crates/periphore-net/Cargo.toml`): Extended from 7 deps to 13: added `periphore-identity`, `periphore-trust`, `periphore-config`, `periphore-core`, `postcard`, `futures-util`. Added `[dev-dependencies]` section with `tempfile` for integration test fixtures.

4. **Root Cargo.toml**: Added `futures-util = { version = "0.3" }` to `[workspace.dependencies]`.

## Tests Added

| Test | File | Covers |
|------|------|--------|
| `ipc_response_pending_peers_round_trip` | `periphore-protocol/tests/roundtrip.rs` | PendingPeerInfo + PendingPeers JSON serialization, type tag, round-trip |
| `daemon_listen_defaults_to_true` | `periphore-config/tests/config.rs` | DaemonConfig::default().listen == true |
| `daemon_listen_can_be_set_false_via_toml` | `periphore-config/tests/config.rs` | TOML listen=false deserialization |
| `daemon_listen_true_when_absent_from_toml` | `periphore-config/tests/config.rs` | serde default triggers when field absent |

**Final test counts:** periphore-protocol: 5/5 pass; periphore-config: 14/14 pass; `cargo build --workspace` exits 0.

## Commits

| Hash | Message |
|------|---------|
| `b54730a` | feat(06-01): add PendingPeerInfo struct and IpcResponse::PendingPeers variant |
| `21d7b87` | feat(06-01): add daemon.listen field and extend periphore-net Cargo.toml |

## Deviations from Plan

None — plan executed exactly as written.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries introduced in this plan. `PendingPeerInfo` is an IPC-local type (Unix domain socket only); no TCP surface added here. Primary threat mitigations (T-6-01: max_frame_length; T-6-06: macOS headless check) are addressed in Plans 02 and 04 as specified.

## Known Stubs

None — no data rendering logic added in this plan. Types are struct/enum definitions only.

## Self-Check: PASSED

- `crates/periphore-protocol/src/ipc.rs`: contains `pub struct PendingPeerInfo` and `PendingPeers { peers: Vec<PendingPeerInfo> }` — FOUND
- `crates/periphore-config/src/schema.rs`: contains `fn default_listen` and `impl Default for DaemonConfig` — FOUND
- `crates/periphore-net/Cargo.toml`: contains `periphore-identity`, `periphore-trust`, `periphore-config`, `periphore-core`, `postcard`, `futures-util` — FOUND
- `Cargo.toml`: contains `futures-util = { version = "0.3" }` — FOUND
- Commit `b54730a`: FOUND
- Commit `21d7b87`: FOUND
