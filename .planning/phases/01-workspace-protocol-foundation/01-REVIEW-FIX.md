---
phase: 01-workspace-protocol-foundation
fixed_at: 2026-04-23T02:59:54Z
review_path: .planning/phases/01-workspace-protocol-foundation/01-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 1: Code Review Fix Report

**Fixed at:** 2026-04-23T02:59:54Z
**Source review:** .planning/phases/01-workspace-protocol-foundation/01-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (WR-01 through WR-05; fix_scope=critical_warning)
- Fixed: 5
- Skipped: 0

## Fixed Issues

### WR-01: Unbounded line reads in IPC server allow memory exhaustion by local user

**Files modified:** `crates/periphore-ipc/src/server.rs`
**Commit:** b950cc5
**Applied fix:** Added `MAX_LINE_BYTES = 64 * 1024` constant before the read loop. After each `read_line` call, the accumulated `line` length is checked against the cap; if exceeded, a warning is logged and the connection is dropped. The check is placed before `trim()` to catch the case where the client never sends a newline.

---

### WR-02: JSON error message in IPC server performs manual string escaping that is incomplete

**Files modified:** `crates/periphore-ipc/src/server.rs`
**Commit:** b950cc5
**Applied fix:** Replaced the manual `format!` + `.replace('"', "'")` construction with `IpcResponse::Error { message: ... }` serialized via `serde_json::to_string`. This is consistent with the success path and correctly handles backslashes, control characters, and all JSON-special bytes. A fallback literal string is used if serialization itself fails.

---

### WR-03: `send_ok` in periphored contains duplicate dispatch for commands already handled in main select

**Files modified:** `crates/periphored/src/main.rs`
**Commit:** 3e1a477
**Applied fix:** Removed the four duplicate arms for `GetStatus`, `InjectInputEvent`, `SimulateEdgeCross`, and `ReloadConfig` from `send_ok`. Added a wildcard `_ => {}` arm (required for Rust match exhaustiveness) with a comment explaining that these variants are always matched in the dedicated `select!` arms and never reach `send_ok`. The incorrect "listed for exhaustiveness" comment has been removed.

---

### WR-04: `PERIPHORE_LOGGING_LEVEL` env var split logic will misparse keys with underscores

**Files modified:** `crates/periphore-config/src/lib.rs`
**Commit:** 15761aa
**Applied fix:** Added a detailed doc comment to the `load()` function explaining that `Env::prefixed("PERIPHORE_").split("_")` splits on every underscore, so field names within config structs must not contain underscores. Includes a concrete example showing `PERIPHORE_DAEMON_SOCKET_PATH` mapping to `daemon.socket.path` (wrong) vs `daemon.socket_path` (intended). Notes `DaemonConfig::socket_path` as the known exempted case and flags it for verification before Phase 5.

---

### WR-05: IPC socket parent directory created with default umask, not hardened permissions

**Files modified:** `crates/periphore-ipc/src/server.rs`
**Commit:** b950cc5
**Applied fix:** After `fs::create_dir_all(parent)`, added a `#[cfg(unix)]` block that calls `fs::set_permissions(parent, fs::Permissions::from_mode(0o700))` to restrict the socket directory to owner-only access. This prevents local users from listing the directory to discover the socket name, complementing the existing 0600 permission on the socket file itself.

---

_Fixed: 2026-04-23T02:59:54Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
