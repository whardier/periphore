---
phase: 01-workspace-protocol-foundation
plan: 05
subsystem: daemon
tags: [rust, tokio, clap, tracing, ipc-wiring, signal-handling, daemon]

# Dependency graph
requires:
  - phase: 01-workspace-protocol-foundation
    provides: periphore-config crate with load() and full schema (Plan 03)
  - phase: 01-workspace-protocol-foundation
    provides: periphore-ipc crate with serve(), IpcCommand, socket path resolver (Plan 04)
provides:
  - "Full periphored daemon binary: config load, IPC socket spawn, GetStatus dispatch, signal handling, clean shutdown"
  - "Wiring of periphore-config and periphore-ipc into a running daemon binary"
  - "periphored --help with --config and --verbose flags via clap v4 derive"
  - "Exhaustive IpcCommand dispatch with placeholder responses for unimplemented commands"
affects: [01-06, phase-02, phase-04, phase-05, phase-06, phase-08, phase-09]

# Tech tracking
tech-stack:
  added: [periphore-protocol dependency in periphored]
  patterns: [tokio-select-event-loop, joinset-task-management, exhaustive-ipccommand-dispatch, send-ok-helper]

key-files:
  created: []
  modified:
    - crates/periphored/src/main.rs
    - crates/periphored/Cargo.toml

key-decisions:
  - "Removed #[cfg(unix)] guards from inside tokio::select! macro arms (unsupported by macro syntax); guards retained on signal variable declarations outside select!"
  - "Added periphore-protocol as direct dependency of periphored for IpcResponse type access"
  - "send_ok() helper uses exhaustive match for compiler-enforced coverage of all IpcCommand variants"

patterns-established:
  - "tokio::select! event loop: signals + IPC commands + task completion in a single loop"
  - "Exhaustive IpcCommand dispatch with send_ok() fallback for unimplemented commands"
  - "JoinSet for spawned task lifecycle management with abort_all on shutdown"

requirements-completed: [IPC-01, IPC-02, CFG-01]

# Metrics
duration: 2min
completed: 2026-04-23
---

# Phase 1 Plan 05: Daemon Wiring Summary

**Full periphored daemon wiring config load, IPC socket spawn, GetStatus dispatch, SIGTERM/SIGHUP handling, and clean shutdown with socket removal**

## Performance

- **Duration:** 2 min
- **Started:** 2026-04-23T02:30:27Z
- **Completed:** 2026-04-23T02:33:06Z
- **Tasks:** 1
- **Files modified:** 3 (main.rs, Cargo.toml, Cargo.lock)

## Accomplishments
- Full daemon entry point replacing stub: config loading via periphore_config::load(), IPC server spawn via periphore_ipc::serve(), GetStatus dispatch with IpcResponse::Status, SIGTERM/SIGHUP signal handling, clean shutdown with socket removal
- periphored --help prints "Periphore input sharing daemon" with --config and --verbose flags
- cargo build --workspace exits 0; cargo clippy -p periphored exits 0 (warnings only, no errors)
- Exhaustive IpcCommand dispatch covering all 12 variants with appropriate placeholder responses

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement full periphored main.rs with config load, IPC wiring, signal handling, and GetStatus dispatch** - `ae27f4a` (feat)

## Files Created/Modified
- `crates/periphored/src/main.rs` - Full daemon main.rs: clap Args, tracing init, config load, IPC socket spawn, signal handling, GetStatus dispatch, exhaustive send_ok() helper, clean shutdown
- `crates/periphored/Cargo.toml` - Added periphore-protocol dependency for IpcResponse type
- `Cargo.lock` - Updated with new dependency resolution

## Decisions Made
- **Removed #[cfg(unix)] from select! arms:** tokio::select! macro does not support `#[cfg(...)]` attributes on match arms. Since this project targets only macOS and Linux (per CLAUDE.md), the `#[cfg(unix)]` guards were moved to only the signal variable declarations outside the select! loop, where they compile correctly. Inside the select! loop, the signal futures are used unconditionally.
- **Added periphore-protocol as direct dependency:** The daemon dispatches IpcCommand variants and constructs IpcResponse values (e.g., IpcResponse::Status, IpcResponse::Ok, IpcResponse::Peers). Since periphore-ipc does not re-export IpcResponse, periphored needs a direct dependency on periphore-protocol.
- **Exhaustive send_ok() helper:** Rather than using a wildcard match arm, the send_ok() function matches every IpcCommand variant exhaustively. This ensures the compiler catches any new variants added to IpcCommand in future phases -- a missing arm becomes a compile error, not a silent bug.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed #[cfg(unix)] attributes from tokio::select! macro arms**
- **Found during:** Task 1 (compilation)
- **Issue:** The plan's code template included `#[cfg(unix)]` on select! arms for signal handlers, but tokio::select! uses a custom macro syntax that does not support `#[cfg(...)]` on arms. Compilation failed with "no rules expected this token in macro call."
- **Fix:** Removed `#[cfg(unix)]` from the two signal handler arms inside select!. The `#[cfg(unix)]` guards remain on the signal variable declarations (lines 64-68), which is sufficient since compilation would fail on non-Unix regardless.
- **Files modified:** `crates/periphored/src/main.rs`
- **Verification:** `cargo build --workspace` exits 0
- **Committed in:** `ae27f4a`

**2. [Rule 3 - Blocking] Added periphore-protocol dependency to periphored Cargo.toml**
- **Found during:** Task 1 (implementation -- main.rs needs IpcResponse type)
- **Issue:** The stub periphored/Cargo.toml did not include periphore-protocol as a dependency, but the daemon constructs IpcResponse values directly (IpcResponse::Status, IpcResponse::Ok, IpcResponse::Peers) for the oneshot responder pattern.
- **Fix:** Added `periphore-protocol = { workspace = true }` to periphored's [dependencies]
- **Files modified:** `crates/periphored/Cargo.toml`
- **Verification:** `cargo build --workspace` exits 0
- **Committed in:** `ae27f4a`

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking)
**Impact on plan:** Both fixes were necessary for compilation. The #[cfg(unix)] removal is cosmetic since the project only targets Unix. The dependency addition was an oversight in the stub Cargo.toml. No scope creep.

## Issues Encountered
None beyond the deviations documented above.

## User Setup Required
None - no external service configuration required.

## Known Stubs
- `SIGHUP` handler logs a reload notice but does not reload config (Phase 4 placeholder)
- `ReloadConfig` IPC command responds with Ok but does not reload (Phase 4)
- `send_ok()` dispatches Ok for most commands that will gain real implementations in later phases
- `fingerprint: None` in GetStatus response (Phase 2 will provide real Ed25519 fingerprint)

All stubs are intentional Phase 1 placeholders documented in the code with TODO/phase references. They do not prevent Phase 1's goal of proving the end-to-end vertical slice works.

## Next Phase Readiness
- Daemon binary fully functional: starts, loads config, creates IPC socket, dispatches commands, handles signals, shuts down cleanly
- Ready for Plan 06 (CLI binary wiring) which is the final plan in Phase 1
- All Phase 1 success criteria met: cargo build --workspace exits 0, IPC socket opens, GetStatus returns response, periphored --help works
- No blockers or concerns

## Self-Check: PASSED

- `crates/periphored/src/main.rs` -- FOUND
- `crates/periphored/Cargo.toml` -- FOUND
- Commit `ae27f4a` (Task 1) -- FOUND
- `cargo build --workspace` -- exits 0
- `./target/debug/periphored --help` -- exits 0, prints "Periphore input sharing daemon"
- `cargo clippy -p periphored` -- exits 0 (warnings only)
- `periphore_config::load` in main.rs -- confirmed
- `periphore_ipc::serve` in main.rs -- confirmed
- `IpcCommand::GetStatus` dispatch -- confirmed
- `sigterm.recv()` signal handler -- confirmed
- `remove_file(&socket_path)` cleanup -- confirmed

---
*Phase: 01-workspace-protocol-foundation*
*Completed: 2026-04-23*
