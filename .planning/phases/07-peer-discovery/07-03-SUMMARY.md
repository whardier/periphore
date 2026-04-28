---
phase: 07-peer-discovery
plan: 03
subsystem: cli
tags: [cli, peers, mdns, discovery, ipc, rust, clap]

# Dependency graph
requires:
  - phase: 07-peer-discovery
    plan: 01
    provides: GetDiscoveredPeers IpcRequest, DiscoveredPeers IpcResponse, GetPendingVerifications IpcRequest, PendingPeers IpcResponse
  - phase: 05-cli-tool
    provides: CLI dispatch pattern (status.rs, trust.rs), ipc_request client helper

provides:
  - periphore peers discovered subcommand: sends GetDiscoveredPeers, displays hostname/port/source/last-seen table with empty-list hint
  - periphore peers pending subcommand: sends GetPendingVerifications, displays fingerprint + word phrase + identicon with trust command hint
  - PeersAction enum (Discovered, Pending) in cli.rs
  - format_age() helper for human-readable relative timestamps

affects: [07-04-PLAN]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - peers subcommand group follows exact Trust { action: TrustAction } pattern from cli.rs
    - handler modules live in commands/peers/ subdirectory, mirroring status.rs/topology.rs pattern
    - format_age() uses SystemTime::now() + UNIX_EPOCH for relative time without external dependencies

key-files:
  created:
    - crates/periphore-cli/src/commands/peers/mod.rs
    - crates/periphore-cli/src/commands/peers/discovered.rs
    - crates/periphore-cli/src/commands/peers/pending.rs
  modified:
    - crates/periphore-cli/src/cli.rs
    - crates/periphore-cli/src/lib.rs
    - crates/periphore-cli/src/commands/mod.rs
    - crates/periphore-cli/tests/cli.rs

key-decisions:
  - "GetPendingVerifications test mock returned IpcResponse::Ok (wrong) — corrected to IpcResponse::PendingPeers { peers: vec![] } in Rule 1 fix"
  - "GetDiscoveredPeers added to test mock router as IpcResponse::DiscoveredPeers { peers: vec![] } — fixes non-exhaustive match from Plan 01 IpcCommand addition"
  - "format_age() uses SystemTime::now() without chrono — avoids new dependency for simple relative formatting"

patterns-established:
  - "Nested subcommand modules go in commands/{name}/ subdirectory with mod.rs declaring sub-modules"
  - "Empty-list hint pattern: check is_empty(), print helpful config snippet, return Ok(())"

requirements-completed: [NET-02]

# Metrics
duration: 2min
completed: 2026-04-28
---

# Phase 07 Plan 03: CLI peers discovered and pending subcommands Summary

**periphore peers discovered (mDNS table with empty-list config hint) and periphore peers pending (fingerprint + word phrase + identicon) wired into CLI dispatch via IPC**

## Performance

- **Duration:** 2 min
- **Started:** 2026-04-28T18:46:08Z
- **Completed:** 2026-04-28T18:48:13Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Added `Peers { action: PeersAction }` variant to `Commands` enum and `PeersAction { Discovered, Pending }` enum in `cli.rs`
- Wired `cli::Commands::Peers` dispatch arm in `lib.rs` routing to `commands::peers::discovered::run` and `commands::peers::pending::run`
- Created `commands/peers/discovered.rs`: sends `GetDiscoveredPeers`, prints formatted HOSTNAME/PORT/SOURCE/LAST SEEN table, empty-list hint includes `[discovery] enabled = true` config snippet
- Created `commands/peers/pending.rs`: sends `GetPendingVerifications`, prints fingerprint + word phrase + identicon per pending peer, includes `periphore trust accept <fingerprint>` hint
- Fixed non-exhaustive match in `tests/cli.rs` mock router (Rule 1 auto-fix) — `GetDiscoveredPeers` and corrected `GetPendingVerifications` response
- `cargo build -p periphore-cli -p periphore` exits 0; all 3 existing tests pass

## Task Commits

1. **Task 1: CLI peers subcommand group and dispatch wiring** - `c633af6` (feat)
2. **Task 2: Discovered and pending CLI command handlers** - `740ae27` (feat)

## Files Created/Modified

- `crates/periphore-cli/src/cli.rs` - Added Peers variant to Commands enum and PeersAction enum
- `crates/periphore-cli/src/lib.rs` - Added Peers dispatch arm routing to discovered/pending handlers
- `crates/periphore-cli/src/commands/mod.rs` - Added `pub(crate) mod peers;` declaration
- `crates/periphore-cli/src/commands/peers/mod.rs` - New module file declaring discovered and pending sub-modules
- `crates/periphore-cli/src/commands/peers/discovered.rs` - Handler for `periphore peers discovered`; table output + empty-list hint + format_age()
- `crates/periphore-cli/src/commands/peers/pending.rs` - Handler for `periphore peers pending`; fingerprint + word phrase + identicon display
- `crates/periphore-cli/tests/cli.rs` - Fixed non-exhaustive match: added GetDiscoveredPeers arm, corrected GetPendingVerifications response

## Decisions Made

- `format_age()` uses `std::time::SystemTime` without `chrono` — relative time for last_seen_epoch was simple enough to implement without adding a dependency
- Test mock `GetPendingVerifications` response corrected from `IpcResponse::Ok` (which was a stub from before the PendingPeers variant existed) to `IpcResponse::PendingPeers { peers: vec![] }` — matches actual daemon behavior

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed non-exhaustive IpcCommand match in test mock router**
- **Found during:** Task 2 (handler verification via `cargo test -p periphore-cli`)
- **Issue:** `crates/periphore-cli/tests/cli.rs::handle_test_command` was missing `IpcCommand::GetDiscoveredPeers` arm (added to `periphore-ipc` in Plan 01) causing E0004 compiler error. Also `IpcCommand::GetPendingVerifications` was returning `IpcResponse::Ok` instead of `IpcResponse::PendingPeers { peers }`.
- **Fix:** Added `IpcCommand::GetDiscoveredPeers { responder }` arm returning `IpcResponse::DiscoveredPeers { peers: vec![] }`; corrected `GetPendingVerifications` to return `IpcResponse::PendingPeers { peers: vec![] }`
- **Files modified:** crates/periphore-cli/tests/cli.rs
- **Verification:** `cargo test -p periphore-cli` passes (3/3 tests)
- **Committed in:** 740ae27 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug — non-exhaustive match + incorrect stub response in test mock)
**Impact on plan:** Required fix for compilation. No scope creep.

## Issues Encountered

None beyond the auto-fixed deviation above.

## Known Stubs

None — both handlers are fully implemented with real IPC calls. The daemon's `GetDiscoveredPeers` dispatch (Plan 04) is the remaining piece to wire the full end-to-end flow.

## Threat Surface Scan

No new network endpoints or trust boundaries introduced. CLI sends IPC requests over the existing Unix socket (0600 owner-only). Threat register T-7-08 and T-7-09 from the plan cover the information disclosure and spoofing considerations for these display-only commands — both accepted.

## Next Phase Readiness

- Plan 04 (daemon wiring: `IpcCommand::GetDiscoveredPeers` dispatch + `DiscoveryService` spawn in `periphored`) can now be executed
- The CLI is ready end-to-end; `periphore peers discovered` will work as soon as Plan 04 wires the daemon side

---
*Phase: 07-peer-discovery*
*Completed: 2026-04-28*
