---
phase: 01-workspace-protocol-foundation
plan: 06
subsystem: cli
tags: [rust, clap, cli-entry, library-stub, workspace-completion]

# Dependency graph
requires:
  - phase: 01-workspace-protocol-foundation
    provides: Cargo workspace scaffold with periphore and periphore-cli crate stubs (Plan 01)
provides:
  - "Finalized periphore CLI binary entry with clap Args, --help/--version output"
  - "periphore-cli library stub with pub fn run() placeholder for Phase 5"
  - "Thin entry + library pattern established: binary delegates to library crate"
  - "Phase 1 complete: cargo build --workspace exits 0 with all 11 crates, both binaries produce --help"
affects: [phase-05, phase-02]

# Tech tracking
tech-stack:
  added: []
  patterns: [thin-binary-entry-delegates-to-library, cli-stub-with-anyhow-bail]

key-files:
  created: []
  modified:
    - crates/periphore/src/main.rs
    - crates/periphore-cli/src/lib.rs

key-decisions:
  - "periphore main.rs uses eprintln! for stub messages (not println!) to keep stdout clean for future structured output"
  - "periphore-cli uses anyhow (not thiserror) because its sole consumer is the periphore binary entry point"

patterns-established:
  - "Thin binary entry: crates/periphore/src/main.rs parses Args then delegates to periphore-cli library"
  - "CLI library stub: pub fn run() -> anyhow::Result with bail! until Phase 5 fills in real dispatch"

requirements-completed: [CFG-01, IPC-01, IPC-02]

# Metrics
duration: 2min
completed: 2026-04-23
---

# Phase 1 Plan 06: CLI Entry & Library Stub Summary

**Finalized periphore CLI binary with clap --help and periphore-cli library stub establishing thin-entry + library delegation pattern for Phase 5**

## Performance

- **Duration:** 2 min
- **Started:** 2026-04-23T02:37:06Z
- **Completed:** 2026-04-23T02:39:21Z
- **Tasks:** 1
- **Files modified:** 2 (main.rs, lib.rs)

## Accomplishments
- periphore CLI binary finalized: clap v4 derive Args with doc comments producing clean --help/--version output
- periphore-cli library stub with pub fn run() placeholder and comprehensive doc comments describing Phase 5 intent
- Both binaries (periphore and periphored) build via plain cargo build (default-members) and via cargo build --workspace
- Phase 1 complete: all 6 plans executed, cargo test --workspace passes, cargo clippy --workspace clean (warnings only)

## Task Commits

Each task was committed atomically:

1. **Task 1: Finalize periphore CLI entry and periphore-cli stub with --help output verified** - `0b6526a` (feat)

## Files Created/Modified
- `crates/periphore/src/main.rs` - Finalized CLI binary entry: clap Args with doc comments describing purpose and relationship to periphored, --help/--version, stub messages via eprintln directing users to periphored
- `crates/periphore-cli/src/lib.rs` - CLI library stub: module-level doc comments, pub fn run() with anyhow::bail placeholder for Phase 5 implementation

## Decisions Made
- **eprintln! for stub messages:** The periphore binary uses eprintln! (not println!) for its "not yet implemented" messages. This keeps stdout clean for future structured output (e.g., JSON status responses) while still informing the user via stderr. This follows the Unix convention of separating data output (stdout) from informational messages (stderr).
- **anyhow in periphore-cli:** The CLI library uses anyhow for error propagation rather than thiserror, because its sole consumer is the periphore binary entry point. This is documented in both the plan and the code. Phase 5 may introduce thiserror for specific error types if needed, but anyhow is appropriate for a CLI support library.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Known Stubs
- `crates/periphore/src/main.rs` line 18: `// Phase 5: periphore_cli::run(args)` -- binary does not yet call the library; prints stub message instead. Phase 5 will wire this.
- `crates/periphore-cli/src/lib.rs` line 15: `anyhow::bail!("periphore-cli: not yet implemented (Phase 5)")` -- library has no real functionality. Phase 5 fills in subcommand dispatch over IPC.

All stubs are intentional Phase 1 placeholders. They do not prevent Phase 1's goal: proving both binary targets are wired into the workspace and produce --help output.

## Phase 1 Retrospective

Phase 1 delivered 6 plans across 4 waves, establishing the entire Cargo workspace foundation:

**Plans that ran smoothly:**
- Plan 01 (workspace scaffold): Clean 11-crate scaffold, all Cargo.toml files wired correctly
- Plan 02 (protocol types): Full PeerMessage enum with postcard round-trip tests
- Plan 06 (this plan): Simple file updates, no complications

**Unexpected complications:**
- Plan 02: `[lib] test = false` on periphore-protocol prevented inline `#[cfg(test)]` modules; all tests moved to `tests/roundtrip.rs` integration test file
- Plan 02: Edge serde format required careful handling for postcard compatibility
- Plan 05: `tokio::select!` macro does not support `#[cfg(unix)]` attributes on match arms; guards moved to signal variable declarations
- Plan 05: periphored needed periphore-protocol as a direct dependency (not re-exported by periphore-ipc)

**Patterns established for future phases:**
- Workspace dependency management: all deps in [workspace.dependencies], crates use { workspace = true }
- Workspace lints: clippy pedantic with selective overrides, priority=-1 for lint group compatibility
- Thin binary entry pattern: binary parses Args, delegates to library crate
- IPC command dispatch: exhaustive match with send_ok() helper for compiler-enforced coverage
- tokio::select! event loop for daemon lifecycle (signals + IPC commands + task completion)
- Config layering: Figment defaults < TOML < env, no Serialize on Config struct (CFG-01)
- Integration test isolation: unique temp socket paths with PID suffix for parallel-safe tests

## Next Phase Readiness
- Phase 1 complete: all 6 success criteria met
- All 11 crates compile, both binaries produce --help, all tests pass
- Ready for Phase 2 (Identity & Cryptography) which fills in periphore-identity
- No blockers or concerns

---
*Phase: 01-workspace-protocol-foundation*
*Completed: 2026-04-23*
