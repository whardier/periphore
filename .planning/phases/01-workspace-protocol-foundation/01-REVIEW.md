---
phase: 01-workspace-protocol-foundation
reviewed: 2026-04-22T00:00:00Z
depth: standard
files_reviewed: 32
files_reviewed_list:
  - .gitignore
  - Cargo.toml
  - crates/periphore-capture/Cargo.toml
  - crates/periphore-capture/src/lib.rs
  - crates/periphore-cli/Cargo.toml
  - crates/periphore-cli/src/lib.rs
  - crates/periphore-config/Cargo.toml
  - crates/periphore-config/src/lib.rs
  - crates/periphore-config/src/schema.rs
  - crates/periphore-config/tests/config.rs
  - crates/periphore-core/Cargo.toml
  - crates/periphore-core/src/lib.rs
  - crates/periphore-identity/Cargo.toml
  - crates/periphore-identity/src/lib.rs
  - crates/periphore-inject/Cargo.toml
  - crates/periphore-inject/src/lib.rs
  - crates/periphore-ipc/Cargo.toml
  - crates/periphore-ipc/src/lib.rs
  - crates/periphore-ipc/src/path.rs
  - crates/periphore-ipc/src/server.rs
  - crates/periphore-ipc/tests/socket.rs
  - crates/periphore-net/Cargo.toml
  - crates/periphore-net/src/lib.rs
  - crates/periphore-protocol/Cargo.toml
  - crates/periphore-protocol/src/ipc.rs
  - crates/periphore-protocol/src/lib.rs
  - crates/periphore-protocol/src/peer.rs
  - crates/periphore-protocol/src/types.rs
  - crates/periphore-protocol/tests/roundtrip.rs
  - crates/periphore/Cargo.toml
  - crates/periphore/src/main.rs
  - crates/periphored/Cargo.toml
  - crates/periphored/src/main.rs
findings:
  critical: 0
  warning: 5
  info: 4
  total: 9
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-04-22T00:00:00Z
**Depth:** standard
**Files Reviewed:** 32
**Status:** issues_found

## Summary

This is the workspace and protocol foundation phase. The codebase is well-structured with a clean Cargo workspace, sensible crate separation, and strong adherence to stated architectural constraints (CFG-01 no-serialize enforced, IPC socket permissions set correctly, no unsafe blocks). Most non-core crates are intentional stubs awaiting future phases.

The substantive implementation lives in four crates: `periphore-config`, `periphore-ipc`, `periphore-protocol`, and `periphored`. Those crates receive the bulk of scrutiny below.

Five warnings were found, all in the IPC and daemon layer. No critical issues (no injection vectors, no hardcoded secrets, no authentication bypasses). Four informational items address minor correctness gaps, naming, and a test robustness concern.

---

## Warnings

### WR-01: Unbounded line reads in IPC server allow memory exhaustion by local user

**File:** `crates/periphore-ipc/src/server.rs:72`

**Issue:** `BufReader::read_line(&mut line)` appends to the same `String` without any size cap. A local process with access to the 0600 socket (i.e., the daemon owner) can send a single line with no newline and of arbitrary length, causing the daemon to grow `line` until OOM. The socket is owner-only, so this is not a remote threat — but it is a local denial-of-service that violates the resilience intent of T-1-02.

**Fix:**
```rust
// After reading, check the accumulated line length before processing.
// Insert this guard at the top of the while loop body:
const MAX_LINE_BYTES: usize = 64 * 1024; // 64 KiB is generous for any IPC request
if line.len() > MAX_LINE_BYTES {
    tracing::warn!("IPC line too long ({} bytes); dropping connection", line.len());
    break;
}
```
Alternatively, replace `read_line` with `take(MAX_LINE_BYTES).read_line(...)` using `AsyncReadExt::take`.

---

### WR-02: JSON error message in IPC server performs manual string escaping that is incomplete

**File:** `crates/periphore-ipc/src/server.rs:133-138`

**Issue:** The malformed-request error path constructs a raw JSON string by calling `.replace('"', "'")` on the serde error message and embedding it via `format!`. This is manual JSON escaping that handles only double-quote characters but leaves backslashes, control characters, and other JSON-special characters unescaped. A serde error message containing a backslash (e.g., from a path in an error context) or a newline would produce malformed JSON sent back to the client.

```rust
// Current (fragile):
let error_json = format!(
    r#"{{"type":"error","message":"malformed request: {}"}}"#,
    e.to_string().replace('"', "'")
);
```

**Fix:** Use `serde_json` to build the error response, which handles escaping correctly:
```rust
// Use the existing IpcResponse::Error variant instead of manual JSON construction:
let response = IpcResponse::Error {
    message: format!("malformed request: {e}"),
};
let mut json = serde_json::to_string(&response)
    .unwrap_or_else(|_| r#"{"type":"error","message":"serialization error"}"#.to_owned());
json.push('\n');
let _ = writer_half.write_all(json.as_bytes()).await;
```
This is consistent with how the success path already serializes `IpcResponse`.

---

### WR-03: `send_ok` in periphored contains duplicate dispatch for commands already handled in main select

**File:** `crates/periphored/src/main.rs:182-225`

**Issue:** `send_ok` contains arms for `GetStatus`, `InjectInputEvent`, `SimulateEdgeCross`, and `ReloadConfig` (lines 209-224) that are explicitly noted as "handled in the main select! arms; listed for exhaustiveness." This is dead code in the control-flow sense: these variants are matched in the `Some(other)` arm only if they fall through from the specific arms above, but they never do because the specific arms are listed first in the `select!` match. If the `match cmd` order in the `select!` is ever changed so that a variant loses its dedicated arm, it will silently fall through to `send_ok` and use the duplicate response — which currently happens to be identical, but is a latent bug that will be hard to notice.

**Fix:** Remove the duplicate arms from `send_ok` and add a `#[allow(unreachable_patterns)]` comment noting why, or restructure to route all IPC dispatch through `send_ok` and remove the duplicates from the `select!`. The comment "These are handled in the main select! arms; listed for exhaustiveness" is incorrect — exhaustiveness in Rust is guaranteed by the compiler, not by duplicate arms.

```rust
// In send_ok, remove:
IpcCommand::GetStatus { responder } => { ... }       // line 209-213
IpcCommand::InjectInputEvent { responder, .. } => {  // line 214-216
IpcCommand::SimulateEdgeCross { responder, .. } => { // line 217-219
IpcCommand::ReloadConfig { responder } => {           // line 220-222
```

---

### WR-04: `PERIPHORE_LOGGING_LEVEL` env var split logic will misparse keys with underscores

**File:** `crates/periphore-config/src/lib.rs:63`

**Issue:** `Env::prefixed("PERIPHORE_").split("_")` splits on every underscore after stripping the prefix. The intent is to map `PERIPHORE_LOGGING_LEVEL` to `logging.level`. This works for the current schema, but the split strategy will break for any future config key whose struct or field name contains an underscore. For example, a future `socket_path` field under `daemon` would require `PERIPHORE_DAEMON_SOCKET_PATH`, which splits to `daemon.socket.path` — a three-level key, not the two-level `daemon.socket_path`. This is a design-time constraint that is not documented and will silently produce "unknown key" errors or fall back to defaults.

**Fix:** Document this constraint in the `load()` function's doc comment and in the schema, and ensure all future field names within a struct avoid underscores, or switch to Figment's `map` provider for more precise key control. Minimum fix for now:

```rust
/// IMPORTANT: Environment variable mapping uses `.split("_")` to produce nested key paths.
/// This means field names within config structs MUST NOT contain underscores, as
/// `PERIPHORE_DAEMON_SOCKET_PATH` would map to `daemon.socket.path` (3 levels), not
/// `daemon.socket_path` (2 levels). All schema fields must use single-word names or this
/// mapping will silently fall back to defaults.
```

Note: `DaemonConfig::socket_path` (line 25, `schema.rs`) is already a field name with an underscore. Verify that `PERIPHORE_DAEMON_SOCKET_PATH` is not expected to configure it via env vars before Phase 5 wires up config overrides.

---

### WR-05: IPC socket parent directory created with default umask, not hardened permissions

**File:** `crates/periphore-ipc/src/server.rs:27-29`

**Issue:** `fs::create_dir_all(parent)` creates the socket directory using the process umask. The socket file itself is set to 0600 (correctly), but the parent directory (e.g., `$TMPDIR/periphore/`) may end up world-executable (e.g., 0755 with a typical 0022 umask), which allows any local user to list the directory, discover the socket name, and attempt to connect. While a 0600 socket refuses unauthorized connections at the kernel level, the defense-in-depth posture and the T-1-03 concern from RESEARCH.md suggests the directory itself should be 0700.

**Fix:**
```rust
if let Some(parent) = socket_path.parent() {
    fs::create_dir_all(parent)?;
    // Harden the directory to 0700 (owner only) for defense in depth.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }
}
```

---

## Info

### IN-01: `unsafe` use of `std::env::set_var`/`remove_var` in tests is undocumented at the call sites

**File:** `crates/periphore-config/tests/config.rs:23, 64, 66`

**Issue:** The `clear_periphore_env()` function uses `unsafe { std::env::remove_var(...) }` and the test `env_overrides_toml_file` uses `unsafe { std::env::set_var(...) }`. The `unsafe` block exists because Rust 1.81+ deprecated the safe versions of these functions due to thread-safety concerns. The `ENV_MUTEX` serializes test execution correctly, but the `unsafe` is not annotated with a `// SAFETY:` comment explaining why the usage is sound, which is the standard Rust convention required to satisfy the `unsafe_code = "warn"` lint declared in the workspace.

**Fix:** Add safety comments at each call site:
```rust
// SAFETY: ENV_MUTEX is held for the duration of this function; no other thread
// can observe or modify PERIPHORE_* env vars while the lock is held.
unsafe { std::env::remove_var("PERIPHORE_LOGGING_LEVEL") };
```

---

### IN-02: `periphore-identity` Cargo.toml disables all tests (`test = false`) without explanation

**File:** `crates/periphore-identity/Cargo.toml:11-12`

**Issue:** `[lib] doctest = false` and `test = false` disable all test and doctest compilation for the crate. The crate is a stub pending Phase 2, but globally suppressing test compilation means that when Phase 2 adds tests, there is a risk the developer forgets to remove these flags and the new tests silently do not run. No comment explains the intent.

**Fix:** Add a comment, or remove the flags now since a stub crate with no tests compiles fine without them:
```toml
[lib]
# doctest and test are disabled because this crate is a Phase 2 stub with no
# implementation. Remove these lines when Phase 2 adds tests.
doctest = false
test    = false
```

---

### IN-03: `periphored` does not log the resolved config log level from `LoggingConfig`

**File:** `crates/periphored/src/main.rs:32-49`

**Issue:** The daemon loads `config.logging.level` from file/env (lines 45-46), logs it (line 48-50), but does not apply it to the `tracing_subscriber` filter. The filter is set based only on `args.verbose` (a boolean `--verbose` flag), not the string level from `config.logging.level`. This means `PERIPHORE_LOGGING_LEVEL=debug` has no effect on the running daemon's log output. The `config.logging.level` field is loaded, logged, and then silently ignored.

**Fix:** Apply the config log level to the filter:
```rust
let log_level = if args.verbose {
    "debug"
} else {
    config.logging.level.as_str()  // Use configured level, not hardcoded "info"
};
// Then build the subscriber with this level as above.
```
Note: This requires loading config before building the subscriber, or building the subscriber twice. A clean approach is to load config first, then initialize logging using the resolved level.

---

### IN-04: IPC socket test cleanup (`remove_file`) runs unconditionally after `abort()`

**File:** `crates/periphore-ipc/tests/socket.rs:142, 159, 190, 216, 238, 258, 283`

**Issue:** Every test calls `server.abort()` then `let _ = std::fs::remove_file(&path)` for cleanup. If the test assertion panics (e.g., `assert!` fails), the cleanup does not run because the panic unwinds past the cleanup lines. The stale socket file may then interfere with other tests that happen to use the same process ID in their path suffix (unlikely but possible in test parallelism). A more robust pattern is a drop guard.

**Fix:** This is low-risk in practice given the unique `pid`-based naming, but for correctness a struct-based cleanup guard would be idiomatic:
```rust
struct SocketCleanup(std::path::PathBuf);
impl Drop for SocketCleanup {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}
// Use as: let _cleanup = SocketCleanup(path.clone());
```

---

_Reviewed: 2026-04-22T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
