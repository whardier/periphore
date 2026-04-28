---
phase: 07-peer-discovery
fixed_at: 2026-04-28T00:00:00Z
review_path: .planning/phases/07-peer-discovery/07-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 7: Code Review Fix Report

**Fixed at:** 2026-04-28T00:00:00Z
**Source review:** .planning/phases/07-peer-discovery/07-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4 (CR-01, WR-01, WR-02, IN-03 upgraded to WR by user)
- Fixed: 4
- Skipped: 0

## Fixed Issues

### CR-01: Any gracefully-completing discovery task triggers full daemon shutdown

**Files modified:** `crates/periphored/src/main.rs`
**Commit:** 3c2f918
**Applied fix:** Moved the IPC server spawn from the shared `JoinSet` into a dedicated `tokio::task::JoinHandle<anyhow::Result<()>>` named `ipc_handle`. The `select!` loop now has two separate branches: `result = &mut ipc_handle` (triggers shutdown on any exit) and `result = tasks.join_next(), if !tasks.is_empty()` (logs but does not shut down). Shutdown path also calls `ipc_handle.abort()` alongside `tasks.abort_all()`. This ensures mDNS task returning `Ok(())` on a corporate network no longer kills the daemon.

---

### WR-01: IPC client `read_line` has no timeout — CLI can hang indefinitely

**Files modified:** `crates/periphore-cli/src/client.rs`
**Commit:** fe158f9
**Applied fix:** Wrapped `reader.read_line(&mut line).await` in `tokio::time::timeout(Duration::from_secs(10), ...)`. On timeout, returns `anyhow::anyhow!("timed out waiting for daemon response (10 s)")`. On I/O error, returns `anyhow::anyhow!("IPC read error: {e}")`. The CLI will now exit with a clear error after 10 seconds instead of blocking forever.

---

### WR-02: mDNS enabled status is logged twice at startup when discovery is enabled

**Files modified:** `crates/periphore-discovery/src/lib.rs`
**Commit:** 2860fa2
**Applied fix:** Removed `tracing::info!("mDNS discovery enabled")` (line 99) and `tracing::info!("SSH tunnel port probing enabled")` (line 119) from `DiscoveryService::start()`. The caller in `periphored/src/main.rs` already emits both messages with richer context (ports list for SSH probe). Replaced the removed log calls with explanatory comments referencing WR-02 and the caller location.

---

### IN-03 (upgraded to WR): mDNS registration failure due to non-unique instance_name and missing .local. suffix on host_name

**Files modified:** `crates/periphore-discovery/src/lib.rs`, `crates/periphore-discovery/src/mdns.rs`
**Commit:** 44ccdd8
**Applied fix:** Two separate bugs fixed together:

1. **instance_name uniqueness** (`lib.rs`): The `unwrap_or_else` fallback for `config.instance_name` now produces `format!("periphore-{}", &identity.fingerprint_hex()[..8])` instead of the literal `"periphore"`. This ensures each host with an unconfigured `instance_name` advertises a distinct name, preventing mDNS name collisions on shared subnets.

2. **host_name .local. suffix** (`mdns.rs`): `ServiceInfo::new()` was passed `""` as `host_name`. The mdns-sd crate's `check_hostname()` (called inside `mdns.register()`) requires the hostname to end with `".local."` and rejects empty strings with "Hostname must end with '.local.'". This caused every registration attempt to fail and fall back to browse-only mode. Fixed by constructing `host_name = format!("{instance_name}.local.")` before calling `ServiceInfo::new()` and passing `&host_name` as the third argument. IP auto-detection via `enable_addr_auto()` is retained.

---

_Fixed: 2026-04-28T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
