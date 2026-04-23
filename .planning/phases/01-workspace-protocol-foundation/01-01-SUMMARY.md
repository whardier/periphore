---
phase: 01-workspace-protocol-foundation
plan: 01
subsystem: infra
tags: [cargo, workspace, rust, scaffold, clippy, lints]

# Dependency graph
requires:
  - phase: none
    provides: greenfield project
provides:
  - "Cargo workspace root with resolver=2, members, default-members"
  - "All 11 crate stubs under crates/ with correct dependency wiring"
  - "Workspace lint policy (clippy pedantic, unsafe_code warn, unreachable_pub warn)"
  - "Two working binary targets: periphore (CLI) and periphored (daemon)"
  - "Cargo.lock committed for reproducible builds"
affects: [01-02, 01-03, 01-04, 01-05, 01-06, phase-02, phase-03, phase-04, phase-05, phase-06, phase-09]

# Tech tracking
tech-stack:
  added: [tokio 1.52, serde 1.0, postcard 1.1, clap 4.6, figment 0.10, tracing 0.1, tracing-subscriber 0.3, thiserror 2.0, anyhow 1.0, ed25519-dalek 2.2, sha2 0.10, directories 6.0, bytes 1.11, tokio-util 0.7, serde_json 1.0]
  patterns: [workspace-deps-with-path-and-version, workspace-lints-inheritance, feature-gating-optional-deps, thin-binary-entry-pattern]

key-files:
  created:
    - Cargo.toml
    - Cargo.lock
    - crates/periphore-protocol/Cargo.toml
    - crates/periphore-protocol/src/lib.rs
    - crates/periphore-config/Cargo.toml
    - crates/periphore-config/src/lib.rs
    - crates/periphore-ipc/Cargo.toml
    - crates/periphore-ipc/src/lib.rs
    - crates/periphore-identity/Cargo.toml
    - crates/periphore-identity/src/lib.rs
    - crates/periphore-core/Cargo.toml
    - crates/periphore-core/src/lib.rs
    - crates/periphore-net/Cargo.toml
    - crates/periphore-net/src/lib.rs
    - crates/periphore-capture/Cargo.toml
    - crates/periphore-capture/src/lib.rs
    - crates/periphore-inject/Cargo.toml
    - crates/periphore-inject/src/lib.rs
    - crates/periphore-cli/Cargo.toml
    - crates/periphore-cli/src/lib.rs
    - crates/periphore/Cargo.toml
    - crates/periphore/src/main.rs
    - crates/periphored/Cargo.toml
    - crates/periphored/src/main.rs
  modified: []

key-decisions:
  - "Clippy pedantic group set to priority=-1 to allow individual lint overrides at default priority (required by lint_groups_priority)"
  - "Cargo.lock committed since workspace produces binary crates"

patterns-established:
  - "Workspace deps: all internal crates declared with path+version in [workspace.dependencies]; crates reference as { workspace = true }"
  - "Workspace lints: [workspace.lints.rust] + [workspace.lints.clippy] in root; all crates have [lints] workspace = true"
  - "Feature gating: periphore-config has optional clap feature; periphore-cli activates it"
  - "Thin binary entry: periphore and periphored are thin main.rs stubs calling into library crates"
  - "Foundational crate override: periphore-protocol and periphore-identity get [lib] doctest=false test=false"

requirements-completed: [CFG-01, IPC-01, IPC-02]

# Metrics
duration: 4min
completed: 2026-04-23
---

# Phase 1 Plan 01: Workspace Scaffold Summary

**Cargo workspace with 11 crates, workspace-level deps/lints, and two binary targets producing --help output**

## Performance

- **Duration:** 4 min
- **Started:** 2026-04-23T01:55:12Z
- **Completed:** 2026-04-23T01:59:32Z
- **Tasks:** 2
- **Files created:** 24

## Accomplishments
- Complete Cargo workspace with resolver=2, 11 crate members, and default-members targeting both binaries
- All 9 internal library crates declared in [workspace.dependencies] with path+version; all external deps at verified versions
- Workspace lint policy active: clippy pedantic (priority=-1), unsafe_code warn, unreachable_pub warn, with selective allow overrides
- Both binaries compile and produce --help output (periphore CLI stub, periphored daemon stub with --config and --verbose flags)
- cargo build --workspace exits 0 with zero errors; cargo clippy --workspace exits 0 (one expected pedantic warning on stub code)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create workspace root Cargo.toml** - `f6b2c2b` (chore)
2. **Task 2: Create all 11 crate stubs** - `8469bbd` (feat)

## Files Created/Modified
- `Cargo.toml` - Workspace root: resolver=2, members, default-members, all workspace deps and lints
- `Cargo.lock` - Lockfile for reproducible builds (123 packages)
- `crates/periphore-protocol/Cargo.toml` - Protocol crate with [lib] doctest=false test=false; deps: serde, postcard, serde_json
- `crates/periphore-protocol/src/lib.rs` - Stub (Plan 02 implements full PeerMessage enum)
- `crates/periphore-config/Cargo.toml` - Config crate with optional clap feature gate; deps: figment, serde, thiserror
- `crates/periphore-config/src/lib.rs` - Stub (Plan 03 implements layered config loading)
- `crates/periphore-ipc/Cargo.toml` - IPC crate; deps: periphore-protocol, tokio, serde_json, directories, thiserror, tracing
- `crates/periphore-ipc/src/lib.rs` - Stub (Plan 04 implements Unix domain socket server)
- `crates/periphore-identity/Cargo.toml` - Identity crate with [lib] doctest=false test=false; deps: ed25519-dalek, sha2, serde
- `crates/periphore-identity/src/lib.rs` - Stub (Phase 2)
- `crates/periphore-core/Cargo.toml` - Core state machine crate; deps: periphore-protocol, serde
- `crates/periphore-core/src/lib.rs` - Stub (Phase 4+)
- `crates/periphore-net/Cargo.toml` - Networking crate; deps: periphore-protocol, tokio, tokio-util, bytes, serde, thiserror, tracing
- `crates/periphore-net/src/lib.rs` - Stub (Phase 6)
- `crates/periphore-capture/Cargo.toml` - Input capture crate; deps: periphore-protocol, tracing
- `crates/periphore-capture/src/lib.rs` - Stub (Phase 9)
- `crates/periphore-inject/Cargo.toml` - Input injection crate; deps: periphore-protocol, tracing
- `crates/periphore-inject/src/lib.rs` - Stub (Phase 9)
- `crates/periphore-cli/Cargo.toml` - CLI library; deps: periphore-config (with clap feature), periphore-ipc, clap, anyhow, tracing
- `crates/periphore-cli/src/lib.rs` - Stub (Phase 5)
- `crates/periphore/Cargo.toml` - CLI binary entry; deps: periphore-cli, clap, anyhow
- `crates/periphore/src/main.rs` - Thin stub with clap Parser, prints placeholder message
- `crates/periphored/Cargo.toml` - Daemon binary entry; deps: periphore-config, periphore-ipc, tokio, clap, tracing, tracing-subscriber, anyhow
- `crates/periphored/src/main.rs` - Stub with clap Parser (--config, --verbose), tokio::main async entry

## Decisions Made
- Clippy pedantic lint group requires `priority = -1` to allow individual lint overrides at default priority; Rust 1.95.0 enforces `lint_groups_priority` by default
- Cargo.lock committed to version control since the workspace produces binary crates (standard Rust practice for reproducible builds)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed clippy lint_groups_priority error**
- **Found during:** Task 2 (cargo clippy --workspace verification)
- **Issue:** Clippy denied build because `pedantic = "warn"` and individual lint overrides (`module_name_repetitions = "allow"`, etc.) had the same priority (0). Rust 1.95.0 enforces `clippy::lint_groups_priority` as `deny` by default.
- **Fix:** Changed `pedantic = "warn"` to `pedantic = { level = "warn", priority = -1 }` in workspace Cargo.toml
- **Files modified:** `Cargo.toml`
- **Verification:** `cargo clippy --workspace` exits 0 after fix
- **Committed in:** `8469bbd` (part of Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Minimal -- the plan's RESEARCH.md specified `pedantic = "warn"` which is correct semantically but needs the priority syntax for Rust 1.95.0. No scope creep.

## Issues Encountered
- One expected clippy pedantic warning (`unnecessary_wraps`) on `crates/periphore/src/main.rs` -- the `anyhow::Result<()>` return type is intentional for the Phase 5 implementation; the stub currently has no error paths. This is a non-issue.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Workspace foundation complete; all 11 crates compile and are wired correctly
- Ready for Plan 02 (protocol types implementation in periphore-protocol)
- No blockers or concerns

## Self-Check: PASSED

- All 24 created files verified present on disk
- Both task commits verified in git log (f6b2c2b, 8469bbd)
- cargo build --workspace exits 0
- Both binaries produce --help output

---
*Phase: 01-workspace-protocol-foundation*
*Completed: 2026-04-23*
