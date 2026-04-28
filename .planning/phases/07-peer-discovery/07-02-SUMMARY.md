---
phase: 07-peer-discovery
plan: 02
subsystem: discovery
tags: [mdns, mdns-sd, ssh-probe, discovery, rust, tokio, periphore-discovery]

# Dependency graph
requires:
  - phase: 07-peer-discovery
    plan: 01
    provides: DiscoveredPeerInfo, DiscoveryConfig, GetDiscoveredPeers IPC types, mdns-sd workspace dep
  - phase: 06-tcp-peering
    provides: periphore-net codec (split_framed, encode_message, decode_message), PROTOCOL_VERSION, DEFAULT_PORT

provides:
  - periphore-discovery crate at crates/periphore-discovery/ (5 source modules)
  - DiscoveryService struct with start(), discovered_list() — spawns mDNS + SSH probe + GC tasks
  - DiscoveryEvent enum (PeerDiscovered, PeerRemoved, Error)
  - DiscoveredPeerList with 64-peer cap, 5-min TTL GC, upsert/remove/remove_by_fullname/gc/snapshot
  - DiscoveryError thiserror enum (MdnsInit, MdnsBrowse, MdnsRegister, Io, Internal)
  - DiscoverySource enum (Mdns, SshProbe) exported at crate root
  - DiscoveryConfig exported from periphore-config (was missing from pub use)

affects: [07-03-PLAN, 07-04-PLAN]

# Tech tracking
tech-stack:
  added: [mdns-sd 0.19 (ServiceDaemon, ServiceInfo, ServiceEvent — register + browse)]
  patterns:
    - mDNS failures log tracing::warn! and continue — daemon never fails on mDNS bind error
    - ServiceEvent is non_exhaustive — Ok(_) wildcard arm required for forward compatibility
    - probe_handshake() disconnects immediately after HelloAck (Pitfall 4 mitigation)
    - own_fingerprint comparison skips self-discovered daemon (Pitfall 3 mitigation)
    - DiscoveredPeerList keyed by "hostname:port" + stores mdns_fullname for ServiceRemoved matching
    - Instant used internally for TTL; converted to u64 epoch seconds only for IPC snapshot (Pitfall 6)
    - GC task always spawned regardless of mDNS/SSH probe config

key-files:
  created:
    - crates/periphore-discovery/Cargo.toml
    - crates/periphore-discovery/src/lib.rs
    - crates/periphore-discovery/src/error.rs
    - crates/periphore-discovery/src/list.rs
    - crates/periphore-discovery/src/mdns.rs
    - crates/periphore-discovery/src/probe.rs
  modified:
    - crates/periphore-config/src/lib.rs

key-decisions:
  - "DiscoveryConfig exported from periphore-config/src/lib.rs — was defined in schema.rs but missing from pub use; added as Rule 3 auto-fix"
  - "ServiceEvent::Ok(_) wildcard arm required — ServiceEvent is #[non_exhaustive] in mdns-sd 0.19"
  - "mdns_register_and_browse() uses browse_loop() helper to share browse path between registration-success and registration-failure (browse-only mode)"
  - "ServiceEvent::ServiceRemoved sends PeerRemoved with port=0 (RFC 6762 goodbye packets don't carry port)"
  - "DiscoveredPeerEntry is pub(crate) not pub — satisfies unreachable_pub lint since it's not exported from lib.rs"
  - "GC task always spawned even when mDNS and SSH probe are disabled — ensures stale entries from previous runs are cleaned"

patterns-established:
  - "periphore-discovery crate follows [lib] test = false pattern (tests go in tests/ subdir)"
  - "mDNS failures are warnings not errors — daemon continues without discovery per CLAUDE.md item 6"
  - "SSH probe uses real Hello/HelloAck handshake for daemon identification (not custom probe)"

requirements-completed: [NET-02]

# Metrics
duration: 4min
completed: 2026-04-28
---

# Phase 07 Plan 02: periphore-discovery Crate Summary

**periphore-discovery crate with mDNS service registration + browse, SSH tunnel port probing via Hello/HelloAck, 64-peer discovered list with TTL GC, and passive DiscoveryService API**

## Performance

- **Duration:** 4 min
- **Started:** 2026-04-28T18:38:06Z
- **Completed:** 2026-04-28T18:42:00Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Created `crates/periphore-discovery/` with all 5 source files: Cargo.toml, error.rs, list.rs, mdns.rs, probe.rs, lib.rs
- `DiscoveredPeerList` enforces 64-peer cap (D-09) with oldest-eviction, 5-minute TTL GC (D-08), and hybrid expiry (D-07) via `remove_by_fullname()` on mDNS goodbye
- `mdns_register_and_browse()` registers local service and browses `_periphore._tcp.local.`; mDNS failures log `tracing::warn!` and continue (CLAUDE.md item 6)
- `ssh_probe_loop()` sweeps configured ports every 30s with 100ms timeout, validates via real Hello/HelloAck, skips self via fingerprint comparison (Pitfall 3)
- `DiscoveryService::start()` spawns mDNS, SSH probe, and GC tasks into the daemon's `JoinSet` using `CancellationToken`
- `cargo build --workspace` green with no errors

## Task Commits

1. **Task 1: Crate scaffold, error types, and discovered peer list** - `429dfa4` (feat)
2. **Task 2: mDNS browse/register, SSH probe loop, and DiscoveryService lib.rs** - `6f2d6b3` (feat)

## Files Created/Modified

- `crates/periphore-discovery/Cargo.toml` - Crate manifest: mdns-sd, periphore-net, periphore-config, periphore-protocol, periphore-identity deps
- `crates/periphore-discovery/src/error.rs` - DiscoveryError with MdnsInit, MdnsBrowse, MdnsRegister, Io, Internal variants
- `crates/periphore-discovery/src/list.rs` - DiscoveredPeerList: upsert/remove/remove_by_fullname/gc/snapshot; MAX_PEERS=64, TTL=300s; DiscoverySource enum
- `crates/periphore-discovery/src/mdns.rs` - mdns_register_and_browse() + browse_loop(); handles ServiceResolved/ServiceRemoved/SearchStarted/ServiceFound/SearchStopped + Ok(_) wildcard
- `crates/periphore-discovery/src/probe.rs` - ssh_probe_loop() with probe_handshake(); PROBE_CONNECT_TIMEOUT=100ms, PROBE_HANDSHAKE_TIMEOUT=200ms, PROBE_INTERVAL=30s; fingerprint self-detection
- `crates/periphore-discovery/src/lib.rs` - DiscoveryService (new, start, discovered_list), DiscoveryEvent enum, GC task always spawned
- `crates/periphore-config/src/lib.rs` - Added DiscoveryConfig to pub use export (Rule 3 auto-fix)

## Decisions Made

- `DiscoveryConfig` exported from `periphore-config/src/lib.rs` — was defined in `schema.rs` but not in `pub use`; required for `periphore_config::DiscoveryConfig` type path in lib.rs to compile (Rule 3 auto-fix)
- `ServiceEvent::Ok(_)` wildcard arm added — `ServiceEvent` is `#[non_exhaustive]` in mdns-sd 0.19, requiring an exhaustive catch-all
- `browse_loop()` extracted as separate function — shares the browse + receive loop between registration-success and registration-failure paths (browse-only mode fallback)
- `ServiceEvent::ServiceRemoved` sends `PeerRemoved { port: 0 }` — RFC 6762 goodbye packets carry only the service fullname, not port; port=0 is the documented sentinel

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Exported DiscoveryConfig from periphore-config**
- **Found during:** Task 2 (lib.rs implementation)
- **Issue:** `periphore_config::DiscoveryConfig` type path failed with "not found in crate `periphore_config`" — `DiscoveryConfig` was defined in `schema.rs` (added in Plan 01) but was not in the `pub use schema::{...}` line in `lib.rs`
- **Fix:** Added `DiscoveryConfig` to the `pub use schema::{...}` export in `crates/periphore-config/src/lib.rs`
- **Files modified:** crates/periphore-config/src/lib.rs
- **Verification:** `cargo build -p periphore-discovery` and `cargo build --workspace` both pass
- **Committed in:** 6f2d6b3 (Task 2 commit)

**2. [Rule 1 - Bug] Added Ok(_) wildcard arm for non-exhaustive ServiceEvent**
- **Found during:** Task 2 (mdns.rs implementation)
- **Issue:** `ServiceEvent` is marked `#[non_exhaustive]` — Rust requires a wildcard arm; compiler error E0004 "non-exhaustive patterns: `Ok(_)` not covered"
- **Fix:** Added `Ok(_) => {}` arm after the known `SearchStopped` arm to handle future mdns-sd ServiceEvent variants gracefully
- **Files modified:** crates/periphore-discovery/src/mdns.rs
- **Verification:** `cargo build -p periphore-discovery` passes
- **Committed in:** 6f2d6b3 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking export, 1 non-exhaustive match)
**Impact on plan:** Both fixes required for compilation correctness. No scope creep.

## Issues Encountered

None beyond the auto-fixed deviations above. Plan executed cleanly once fixes were applied.

## Known Stubs

None — all 5 source modules are fully implemented. The GC, mDNS browse, and SSH probe tasks have complete logic. The `identity.keypair` direct access is documented as a consequence of WR-01 (an existing open TODO from Phase 6), not a new stub.

## Next Phase Readiness

- Plan 03 (daemon wiring) can now import `periphore_discovery::{DiscoveryService, DiscoveryEvent}` and call `service.start()` into the daemon's `JoinSet`
- Plan 03 needs to add `IpcCommand::GetDiscoveredPeers` dispatch arm calling `discovery_service.discovered_list()`
- Plan 04 (CLI) has all IPC types it needs for `periphore peers discovered`

---
*Phase: 07-peer-discovery*
*Completed: 2026-04-28*
