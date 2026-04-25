---
phase: 05-cli-tool
plan: 01
subsystem: cli
tags: [rust, clap, tokio, serde_json, unix-socket, ipc, anyhow]

# Dependency graph
requires:
  - phase: 04-ipc-layer
    provides: periphore-ipc with serve(), IpcCommand, socket path utilities
  - phase: 01-workspace
    provides: periphore-protocol with IpcRequest/IpcResponse types

provides:
  - periphore-cli Cargo.toml with tokio, serde_json, periphore-protocol workspace deps
  - cli.rs with pub Cli (Parser) and pub Commands (Subcommand) types
  - client.rs with pub async ipc_request() and daemon_not_running_error()

affects:
  - 05-02 (wires lib.rs, adds commands/status.rs and commands/topology.rs)
  - 05-03 (wires periphore/src/main.rs entry point calling periphore_cli::run)

# Tech tracking
tech-stack:
  added:
    - tokio (workspace dep added to periphore-cli)
    - serde_json (workspace dep added to periphore-cli)
    - periphore-protocol (workspace dep added to periphore-cli)
  patterns:
    - JSON-lines IPC client: BufReader::read_line + serde_json::from_str::<T>(line.trim())
    - ErrorKind-based error classification over string matching for ENOENT/ECONNREFUSED
    - global = true clap args for --socket/--config that work at any subcommand position
    - Parsing deferred to binary entry point (Cli::parse() never called in library crate)

key-files:
  created:
    - crates/periphore-cli/src/cli.rs
    - crates/periphore-cli/src/client.rs
  modified:
    - crates/periphore-cli/Cargo.toml

key-decisions:
  - "global = true on --socket and --config ensures flag works before any subcommand position — clap requires this for global args"
  - "daemon_not_running_error uses e.kind() match on ErrorKind (not e.to_string()) — fragile string matching avoided"
  - "ipc_request uses anyhow::Result throughout — no .unwrap() or .expect() on IPC operations"
  - "cli.rs is a library module — Cli::parse() deferred to periphore/src/main.rs (thin entry point)"

patterns-established:
  - "JSON-lines IPC client pattern: serialize IpcRequest, push '\\n', write_all, read_line, trim, deserialize IpcResponse"
  - "Error classification: ErrorKind::NotFound -> socket missing; ErrorKind::ConnectionRefused -> daemon crashed; _ -> unexpected"
  - "No tracing_subscriber initialization in periphore-cli (D-26 constraint — only periphored initializes subscriber)"

requirements-completed: [TOP-04]

# Metrics
duration: 2min
completed: 2026-04-25
---

# Phase 5 Plan 01: CLI Foundation Summary

**Clap Cli/Commands types and async ipc_request() transport over UnixStream with ErrorKind-based daemon-not-running error classification**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-04-25T15:35:25Z
- **Completed:** 2026-04-25T15:37:14Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments

- Added three workspace deps (tokio, serde_json, periphore-protocol) to periphore-cli Cargo.toml
- Created cli.rs with pub Cli (global --socket/--config args + Commands subcommand) and pub Commands enum (Status, Topology) — no parse() call
- Created client.rs with pub async ipc_request() using JSON-lines framing and daemon_not_running_error() mapping ENOENT/ECONNREFUSED to user-friendly messages

## Task Commits

Each task was committed atomically:

1. **Task 1: Add missing workspace deps to periphore-cli/Cargo.toml** - `12cb066` (feat)
2. **Task 2: Create cli.rs — Cli struct and Commands enum** - `6429828` (feat)
3. **Task 3: Create client.rs — async ipc_request() transport function** - `ff1ad5d` (feat)

## Files Created/Modified

- `crates/periphore-cli/Cargo.toml` - added tokio, serde_json, periphore-protocol workspace deps
- `crates/periphore-cli/src/cli.rs` - pub Cli (Parser) with global --socket/--config; pub Commands enum (Status, Topology)
- `crates/periphore-cli/src/client.rs` - pub async ipc_request(), daemon_not_running_error() with ErrorKind classification

## Decisions Made

- `global = true` on `--socket` and `--config` in Cli struct so both args work regardless of subcommand position — required for pre-subcommand flag parsing in clap
- `daemon_not_running_error` uses `e.kind()` match on `ErrorKind` enum values rather than string comparison — more robust across locales and Rust versions
- `Cli::parse()` is intentionally absent from cli.rs — library crates should not call parse(); the binary entry point (`crates/periphore/src/main.rs`) does this in Plan 03

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None — cargo check passes cleanly after all three tasks.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 02 can now import `pub mod cli` and `pub mod client` in lib.rs, and build `commands/status.rs` and `commands/topology.rs` on top of `ipc_request()`
- Plan 03 can wire `crates/periphore/src/main.rs` as a thin `Cli::parse()` + `periphore_cli::run()` entry point
- All three must_haves/truths satisfied: periphore-cli compiles with tokio/serde_json/periphore-protocol; Cli/Commands defined and exported; ipc_request() connects UnixStream, writes JSON-line, reads JSON-line, maps ENOENT/ECONNREFUSED

---
*Phase: 05-cli-tool*
*Completed: 2026-04-25*
