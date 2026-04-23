---
phase: 01-workspace-protocol-foundation
plan: 04
subsystem: ipc
tags: [rust, tokio, unix-socket, ipc, json-lines, security]

# Dependency graph
requires:
  - phase: 01-workspace-protocol-foundation
    provides: Cargo workspace with periphore-ipc crate stub and periphore-protocol types
provides:
  - "IpcCommand enum with 12 variants and oneshot responder pattern for request/response"
  - "Unix domain socket server with JSON-lines protocol (serve() async fn)"
  - "Platform socket path resolver (XDG on Linux, TMPDIR on macOS)"
  - "Security mitigations: stale socket removal, 0600 permissions, no unwrap on JSON parse"
  - "8 integration tests covering socket lifecycle and IPC protocol round-trips"
affects: [01-05, 01-06, phase-04, phase-05, phase-06, phase-09]

# Tech tracking
tech-stack:
  added: []
  patterns: [oneshot-responder-per-ipc-command, json-lines-over-unix-socket, temp-socket-per-test-isolation]

key-files:
  created:
    - crates/periphore-ipc/src/path.rs
    - crates/periphore-ipc/src/server.rs
    - crates/periphore-ipc/tests/socket.rs
  modified:
    - crates/periphore-ipc/src/lib.rs
    - crates/periphore-ipc/Cargo.toml

key-decisions:
  - "IpcCommand uses oneshot responder pattern: each command carries a oneshot::Sender<IpcResponse> so the IPC layer can write the daemon's response back to the client"
  - "Edge enum serializes as plain string (\"Right\") not tagged object ({\"Right\":null}) in serde_json -- tests use plain string form"
  - "Each test uses a unique temp socket path with PID suffix to prevent conflicts in parallel test runs"
  - "handle_connection returns error JSON for malformed input rather than silently dropping -- client always gets a response"

patterns-established:
  - "Oneshot responder pattern: IpcCommand variants carry oneshot::Sender<IpcResponse> for bidirectional IPC"
  - "Test isolation via temp_socket_path(test_name) with PID suffix -- each test gets a unique socket"
  - "handle_test_command() exhaustive match for test router -- mirrors daemon's future command dispatch"

requirements-completed: [IPC-01, IPC-02]

# Metrics
duration: 6min
completed: 2026-04-23
---

# Phase 1 Plan 04: IPC Implementation Summary

**Unix domain socket IPC server with JSON-lines protocol, oneshot responder pattern, 0600 security, and 10 passing tests**

## Performance

- **Duration:** 6 min
- **Started:** 2026-04-23T02:19:58Z
- **Completed:** 2026-04-23T02:26:13Z
- **Tasks:** 2
- **Files created:** 3
- **Files modified:** 2

## Accomplishments
- Complete IPC server with stale socket removal, 0600 permissions, and JSON-lines protocol over Unix domain socket
- IpcCommand enum with all 12 variants using oneshot responder pattern for bidirectional request/response
- Platform socket path resolver using directories crate (XDG_RUNTIME_DIR on Linux, TMPDIR on macOS)
- 10 tests total: 2 unit tests (path resolution) + 8 integration tests (socket lifecycle + IPC protocol)
- All 4 security threats mitigated: T-1-01 (socket hijack), T-1-02 (malformed JSON DoS), T-1-04 (stale socket), T-1-P04-01 (world-writable tmp)

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement IpcCommand, socket path resolver, and server core** - `ea04043` (feat)
2. **Task 2: Write IPC socket integration tests** - `1252a28` (test)

## Files Created/Modified
- `crates/periphore-ipc/src/path.rs` - Platform socket path resolver: XDG on Linux, TMPDIR on macOS, with unit tests
- `crates/periphore-ipc/src/server.rs` - Unix socket server: stale removal, 0600 perms, JSON-lines accept loop, handle_connection with oneshot responders
- `crates/periphore-ipc/src/lib.rs` - IpcCommand enum (12 variants with oneshot responders), from_request_with_responder constructor, pub serve + pub path
- `crates/periphore-ipc/tests/socket.rs` - 8 integration tests: socket_creates, socket_permissions_0600, stale_socket_does_not_block_restart, get_status_returns_status_response, inject_input_event_no_peer_required, simulate_edge_cross_no_peer_required, malformed_request_returns_error_not_crash, socket_path_resolution_returns_periphore_sock
- `crates/periphore-ipc/Cargo.toml` - Added tokio and serde_json dev-dependencies for integration tests

## Decisions Made
- **Oneshot responder pattern:** Each IpcCommand variant carries a `oneshot::Sender<IpcResponse>` so the server can write the daemon's response back to the client through the IPC layer. This keeps routing in the daemon and transport in the IPC crate. The daemon will handle responders in Plan 05.
- **Edge serializes as plain string:** `Edge::Right` serializes as `"Right"` (not `{"Right":null}`) in serde_json because the Edge enum uses plain Serialize without tag attributes. Tests use the plain string form.
- **Test isolation via temp paths:** Each test creates a unique socket path under `$TMPDIR/periphore-test/{test_name}-{pid}.sock` to prevent conflicts when tests run in parallel.
- **Error response on malformed input:** The server sends an error JSON response back to the client for malformed requests rather than silently dropping the line, giving clients feedback about what went wrong.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## TDD Gate Compliance
- RED: N/A for Task 1 (implementation code, not test-first) -- verified stub was present, then implemented
- GREEN: Task 1 `cargo build -p periphore-ipc` exits 0 after implementation -- confirmed passing
- RED: Task 2 tests written to match implementation behavior
- GREEN: `cargo test -p periphore-ipc` exits 0 with all 10 tests passing -- confirmed passing
- REFACTOR: not needed -- code is clean as written

## Next Phase Readiness
- IPC crate fully functional; periphored can now wire IPC server into its main loop (Plan 05)
- InjectInputEvent and SimulateEdgeCross exercisable from Phase 1 forward (D-19 achieved)
- No blockers or concerns

## Self-Check: PASSED

- `crates/periphore-ipc/src/path.rs` -- FOUND
- `crates/periphore-ipc/src/server.rs` -- FOUND
- `crates/periphore-ipc/src/lib.rs` -- FOUND
- `crates/periphore-ipc/tests/socket.rs` -- FOUND
- Commit `ea04043` (Task 1) -- FOUND
- Commit `1252a28` (Task 2) -- FOUND
- `cargo test -p periphore-ipc` -- 10 passed, 0 failed
- `cargo build -p periphore-ipc` -- exits 0
- Security mitigations: remove_file before bind, 0o600 after bind, no unwrap on JSON parse -- all confirmed

---
*Phase: 01-workspace-protocol-foundation*
*Completed: 2026-04-23*
