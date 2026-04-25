---
phase: 05-cli-tool
plan: 02
subsystem: cli
tags: [rust, clap, tokio, anyhow, unix-socket, ipc, commands]

# Dependency graph
requires:
  - phase: 05-01
    provides: cli.rs (Cli/Commands), client.rs (ipc_request), periphore-cli Cargo.toml deps
  - phase: 04-ipc-layer
    provides: periphore-ipc serve(), IpcCommand, socket path utilities
  - phase: 01-workspace
    provides: periphore-protocol IpcRequest/IpcResponse types

provides:
  - commands/mod.rs declaring pub(crate) mod status and pub(crate) mod topology
  - commands/status.rs: GetStatus -> IpcResponse::Status output handler
  - commands/topology.rs: GetTopology -> IpcResponse::Ok graceful stub handler
  - lib.rs: pub async fn run(cli: Cli), resolve_socket_path(), pub use Cli
  - periphore/src/main.rs: #[tokio::main] thin entry point calling periphore_cli::run
  - periphore/Cargo.toml: tokio workspace dep added

affects:
  - 05-03 (all wiring complete; 05-03 was folded into this plan)

# Tech tracking
tech-stack:
  added:
    - tokio (workspace dep added to periphore binary crate for #[tokio::main])
  patterns:
    - Command handler pattern: pub(crate) async fn run(socket_path: &Path) -> anyhow::Result<()>
    - Three-tier socket path resolution: --socket flag > config.daemon.socket_path > periphore_ipc::path::socket_path()
    - Graceful stub handling: IpcResponse::Ok for GetTopology is success (not an error) until Phase 8
    - Wildcard IPC response arm uses tracing::debug!(?other, ...) not println!("{other:?}") (satisfies pedantic clippy)
    - pub(crate) visibility on commands submodules satisfies unreachable_pub lint

key-files:
  created:
    - crates/periphore-cli/src/commands/mod.rs
    - crates/periphore-cli/src/commands/status.rs
    - crates/periphore-cli/src/commands/topology.rs
  modified:
    - crates/periphore-cli/src/lib.rs
    - crates/periphore/src/main.rs
    - crates/periphore/Cargo.toml

key-decisions:
  - "pub(crate) used on commands submodules and run() functions — unreachable_pub pedantic lint requires non-pub visibility for items not exported from the crate"
  - "tokio added to periphore/Cargo.toml — #[tokio::main] macro expands to tokio::runtime code so the binary crate needs it as a direct dep"
  - "resolve_socket_path silently ignores config load failures — periphore must work without a config file (first-run UX)"
  - "IpcResponse::Ok in topology.rs is success not error — daemon stubs GetTopology with Ok until Phase 8 delivers real topology variant"

# Metrics
duration: ~2.5min
completed: 2026-04-25
---

# Phase 5 Plan 02: CLI Command Dispatch Summary

**Command handlers (status/topology), lib.rs run() dispatch, and periphore/src/main.rs #[tokio::main] entry — periphore status and periphore topology fully wired end-to-end**

## Performance

- **Duration:** ~2.5 min
- **Started:** 2026-04-25T15:39:54Z
- **Completed:** 2026-04-25T15:42:19Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Created commands/mod.rs, commands/status.rs, commands/topology.rs with correct IPC request/response handling
- Replaced lib.rs stub with real pub async fn run(cli: Cli), module declarations, pub use Cli, and resolve_socket_path()
- Replaced periphore/src/main.rs stub with 3-line #[tokio::main] entry calling periphore_cli::run(Cli::parse()).await
- Added tokio to periphore/Cargo.toml (required for #[tokio::main] macro expansion)
- cargo build --workspace exits 0 — no warnings, no errors

## Task Commits

Each task was committed atomically:

1. **Task 1: Create commands/ module with status and topology handlers** - `8bb0a8d` (feat)
2. **Task 2: Wire lib.rs run() dispatch and tokio::main entry point** - `506e527` (feat)

## Files Created/Modified

- `crates/periphore-cli/src/commands/mod.rs` - declares pub(crate) mod status and pub(crate) mod topology
- `crates/periphore-cli/src/commands/status.rs` - GetStatus -> IpcResponse::Status with Daemon/Fingerprint output
- `crates/periphore-cli/src/commands/topology.rs` - GetTopology -> IpcResponse::Ok treated as graceful stub (not error)
- `crates/periphore-cli/src/lib.rs` - pub mod cli/client, mod commands, pub use Cli, pub async fn run(), resolve_socket_path()
- `crates/periphore/src/main.rs` - #[tokio::main] async fn main() calling periphore_cli::run(Cli::parse()).await
- `crates/periphore/Cargo.toml` - added tokio workspace dep

## Decisions Made

- `pub(crate)` visibility on command submodules and their `run()` functions — the pedantic `unreachable_pub` lint rejects `pub` items that are not reachable from outside the crate; `commands` module is `mod` (not `pub mod`) in lib.rs so its contents must be `pub(crate)`
- `tokio` added as direct dep to `periphore/Cargo.toml` — `#[tokio::main]` is a proc-macro that emits `tokio::runtime::Builder` code into main.rs; the binary crate needs tokio as a direct dep for that expansion to resolve
- Config load failure in `resolve_socket_path` is silently ignored — daemon's first-run behavior is to work without a config file; CLI must match

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added tokio dep to periphore/Cargo.toml**
- **Found during:** Task 2 build (`cargo build -p periphore`)
- **Issue:** `#[tokio::main]` macro requires `tokio` as a direct dependency of the binary crate; plan listed it only for periphore-cli
- **Fix:** Added `tokio = { workspace = true }` to `crates/periphore/Cargo.toml`
- **Files modified:** `crates/periphore/Cargo.toml`
- **Commit:** 506e527

**2. [Rule 2 - Lint] Changed pub to pub(crate) on commands submodule items**
- **Found during:** Task 2 build (unreachable_pub pedantic warnings treated as errors by workspace lints)
- **Issue:** `commands` is declared `mod commands` (not `pub mod`) in lib.rs; items inside must be `pub(crate)` not `pub` to satisfy the unreachable_pub lint
- **Fix:** Changed `pub mod status/topology` to `pub(crate) mod status/topology` in mod.rs; changed `pub async fn run` to `pub(crate) async fn run` in status.rs and topology.rs
- **Files modified:** commands/mod.rs, commands/status.rs, commands/topology.rs
- **Commit:** 506e527

## Known Stubs

- `periphore topology` prints "Topology: not yet available" — this is intentional per plan design; IpcResponse::Ok is the daemon's correct stub response until Phase 8 adds a real Topology variant. Not a defect.

## Threat Surface Scan

No new trust boundaries introduced beyond those documented in the plan's threat model (T-5-01, T-5-02, T-5-03 all addressed in implementation).

## Self-Check: PASSED

- `crates/periphore-cli/src/commands/mod.rs` — FOUND
- `crates/periphore-cli/src/commands/status.rs` — FOUND
- `crates/periphore-cli/src/commands/topology.rs` — FOUND
- `crates/periphore-cli/src/lib.rs` — FOUND (updated)
- `crates/periphore/src/main.rs` — FOUND (updated)
- `crates/periphore/Cargo.toml` — FOUND (updated)
- Commit `8bb0a8d` — FOUND
- Commit `506e527` — FOUND
- `cargo build --workspace` — exits 0

---
*Phase: 05-cli-tool*
*Completed: 2026-04-25*
