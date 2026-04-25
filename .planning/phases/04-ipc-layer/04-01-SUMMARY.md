---
phase: 04-ipc-layer
plan: 01
subsystem: daemon
tags: [tokio, joinset, select, cpu-spin, cr-01]

requires:
  - phase: 01-workspace-protocol-foundation
    provides: periphored main.rs with JoinSet task management and select! loop

provides:
  - JoinSet empty-set guard eliminating 100% CPU spin (CR-01 fixed)
  - Clean IPC server exit triggers daemon shutdown via break

affects: [periphored, 04-02, 04-03]

tech-stack:
  added: []
  patterns:
    - "tokio::select! precondition guard: `result = future, if condition => { ... }` disables branch when condition is false"

key-files:
  created: []
  modified:
    - crates/periphored/src/main.rs

key-decisions:
  - "CR-01: add `, if !tasks.is_empty()` guard to join_next branch — disables the branch when JoinSet is empty, preventing Poll::Ready(None) from spinning the select! loop at 100% CPU"
  - "CR-01: add break on Some(Ok(Ok(()))) arm — clean IPC server exit shuts down the daemon rather than leaving it alive with an empty JoinSet"

patterns-established:
  - "JoinSet precondition guard pattern: always guard join_next() branches with `if !tasks.is_empty()` to avoid CPU spin"

requirements-completed: [IPC-01, IPC-02]

duration: 5min
completed: 2026-04-25
---

# Phase 4 Plan 01: CR-01 JoinSet CPU Spin Fix Summary

**Two-line surgical fix: `if !tasks.is_empty()` guard + `break` on clean exit eliminates 100% CPU spin when periphored's IPC task exits cleanly**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-04-25T00:00:00Z
- **Completed:** 2026-04-25T00:05:00Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Eliminated CR-01: JoinSet::join_next() on an empty set returns Poll::Ready(None) immediately, causing the tokio::select! branch to win every iteration and spin the loop at 100% CPU. The `if !tasks.is_empty()` precondition disables the branch entirely when there are no tasks.
- Added `break` on `Some(Ok(Ok(())))` so that a clean IPC server exit triggers daemon shutdown rather than leaving periphored alive with no tasks and no way to be reached.
- Updated the None arm comment to document that it is now unreachable given the empty-set guard (kept for defensive coverage).
- All 46 workspace tests pass, build succeeds.

## Task Commits

1. **Task 1: Fix JoinSet spin — add empty-guard and clean-exit break** - `a683eb6` (fix)

## Files Created/Modified

- `/Users/spencersr/src/github/whardier/periphore/crates/periphored/src/main.rs` - Added `, if !tasks.is_empty()` precondition to join_next branch; added `break` and updated log message on clean-exit arm; updated None arm comment

## Decisions Made

None - followed plan exactly as specified. Both changes were prescribed by the plan (D-11, D-12 from 04-CONTEXT.md).

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- CR-01 is resolved; periphored no longer spins at 100% CPU when the IPC task exits
- Ready for 04-02 (full config reload) and 04-03 (periphore-core state machine)
- Open TODOs WR-01, WR-02, WR-03, IN-03 remain deferred (logged in STATE.md, out of scope for this plan)

---

## Self-Check: PASSED

- `crates/periphored/src/main.rs` contains `, if !tasks.is_empty()` — FOUND
- `crates/periphored/src/main.rs` contains `"IPC server task completed -- shutting down"` — FOUND
- Commit `a683eb6` exists — FOUND
- `cargo build --workspace` exits 0 — PASSED
- `cargo test --workspace` exits 0 (46 tests, 0 failures) — PASSED

---
*Phase: 04-ipc-layer*
*Completed: 2026-04-25*
