---
phase: 07-peer-discovery
reviewed: 2026-04-28T00:00:00Z
depth: standard
files_reviewed: 24
files_reviewed_list:
  - Cargo.toml
  - crates/periphore-cli/src/cli.rs
  - crates/periphore-cli/src/commands/mod.rs
  - crates/periphore-cli/src/commands/peers/discovered.rs
  - crates/periphore-cli/src/commands/peers/mod.rs
  - crates/periphore-cli/src/commands/peers/pending.rs
  - crates/periphore-cli/src/lib.rs
  - crates/periphore-cli/tests/cli.rs
  - crates/periphore-config/src/lib.rs
  - crates/periphore-config/src/schema.rs
  - crates/periphore-discovery/Cargo.toml
  - crates/periphore-discovery/src/error.rs
  - crates/periphore-discovery/src/lib.rs
  - crates/periphore-discovery/src/list.rs
  - crates/periphore-discovery/src/mdns.rs
  - crates/periphore-discovery/src/probe.rs
  - crates/periphore-discovery/tests/integration.rs
  - crates/periphore-ipc/src/lib.rs
  - crates/periphore-ipc/tests/socket.rs
  - crates/periphore-protocol/src/ipc.rs
  - crates/periphore-protocol/src/lib.rs
  - crates/periphore-protocol/tests/roundtrip.rs
  - crates/periphored/Cargo.toml
  - crates/periphored/src/main.rs
findings:
  critical: 1
  warning: 2
  info: 4
  total: 7
status: issues_found
---

# Phase 7: Code Review Report

**Reviewed:** 2026-04-28T00:00:00Z
**Depth:** standard
**Files Reviewed:** 24
**Status:** issues_found

## Summary

Phase 7 introduces `periphore-discovery` (mDNS registration/browsing and SSH tunnel port
probing), extends the IPC protocol with `GetDiscoveredPeers` / `GetPendingVerifications`,
adds `periphore peers discovered` and `periphore peers pending` CLI subcommands, and wires
the discovery service into the `periphored` daemon main loop.

The implementation is generally clean and well-structured. The mDNS graceful-degradation
path (warn and continue), TCP_NODELAY enforcement in the probe, self-detection via
fingerprint comparison, and the Instant-to-epoch conversion are all handled correctly.

However, one critical bug will cause the daemon to shut itself down silently whenever the
mDNS subsystem exits gracefully (e.g., on a network where mDNS is unavailable). Two
warning-level issues could cause the CLI to hang indefinitely under certain daemon failure
modes and introduce duplicate log noise. Four info-level items cover test infrastructure
gaps and code duplication.

---

## Critical Issues

### CR-01: Any gracefully-completing discovery task triggers full daemon shutdown

**File:** `crates/periphored/src/main.rs:427-445`

**Issue:** The `tasks.join_next()` branch in the main `select!` loop treats *any* task
completing with `Ok(Ok(()))` as the IPC server completing, then logs "IPC server task
completed -- shutting down" and breaks out of the loop. All tasks — the IPC server, mDNS
browse loop, SSH probe loop, and GC task — are spawned into the same `JoinSet`. When mDNS
is unavailable on the network, `mdns_register_and_browse` returns `Ok(())` at line 38 of
`crates/periphore-discovery/src/mdns.rs`. This yields `Some(Ok(Ok(())))` from
`join_next()`, triggering daemon shutdown. The same applies to any discovery task that
exits cleanly: the SSH probe loop returns `Ok(())` on cancellation (which happens during
the cancellation sequence itself), and the GC task does likewise. In practice, enabling
mDNS on a corporate network — the exact scenario documented in CLAUDE.md as a known
pitfall — will silently take down the whole daemon immediately after startup.

**Fix:** Tag tasks by category so graceful exit from a non-critical task is handled
differently from IPC server exit. The minimal fix is to track whether the IPC server task
has completed separately. One approach uses a dedicated `JoinHandle` for the IPC server
and a separate `JoinSet` for discovery tasks:

```rust
// Spawn IPC server into a dedicated handle, not the shared JoinSet
let ipc_handle: tokio::task::JoinHandle<anyhow::Result<()>> = {
    let ipc_path = socket_path.clone();
    tokio::spawn(async move {
        periphore_ipc::serve(&ipc_path, ipc_cmd_tx)
            .await
            .map_err(|e| anyhow::anyhow!("IPC server error: {e}"))
    })
};

// Discovery and other background tasks remain in `tasks` JoinSet.
// In the select! loop, poll `ipc_handle` directly:

result = &mut ipc_handle => {
    match result {
        Ok(Ok(())) => { tracing::info!("IPC server task completed -- shutting down"); break; }
        Ok(Err(e)) => { tracing::error!("IPC server error: {e}"); break; }
        Err(e)     => { tracing::error!("IPC server panicked: {e}"); break; }
    }
}

// For the shared JoinSet, log non-critical task completions without shutting down:
result = tasks.join_next(), if !tasks.is_empty() => {
    match result {
        Some(Ok(Ok(()))) => { tracing::debug!("background task completed normally"); }
        Some(Ok(Err(e))) => { tracing::warn!("background task error: {e}"); }
        Some(Err(e))     => { tracing::error!("background task panicked: {e}"); }
        None             => {}
    }
}
```

---

## Warnings

### WR-01: IPC client `read_line` has no timeout — CLI can hang indefinitely

**File:** `crates/periphore-cli/src/client.rs:35`

**Issue:** `reader.read_line(&mut line).await?` has no timeout. If the daemon accepts the
connection (so the socket connect succeeds) but the responder oneshot is dropped before
sending a response — for example due to a deadlock, panic in the routing task, or a future
`IpcCommand` variant that falls through `send_ok` with `_ => {}` — the CLI process will
block forever. This is especially problematic for scripted use (`periphore peers discovered
&& do_something`).

**Fix:** Wrap the read in `tokio::time::timeout`:

```rust
let mut line = String::new();
tokio::time::timeout(
    std::time::Duration::from_secs(10),
    reader.read_line(&mut line),
)
.await
.map_err(|_| anyhow::anyhow!("timed out waiting for daemon response"))?
.map_err(|e| anyhow::anyhow!("IPC read error: {e}"))?;
```

A 10-second timeout is generous for local Unix socket IPC; 5 seconds is also reasonable.

---

### WR-02: mDNS enabled status is logged twice at startup when discovery is enabled

**File:** `crates/periphore-discovery/src/lib.rs:99` and `crates/periphored/src/main.rs:191`

**Issue:** When `config.enabled` is true, both `DiscoveryService::start()` (inside the
`if config.enabled` block at line 99 of `lib.rs`) and the caller in `main.rs` (line 191)
emit `tracing::info!("mDNS discovery enabled")`. Operators viewing the daemon log will see
the message duplicated on every startup with mDNS enabled, which looks like a bug or a
logging loop.

**Fix:** Remove the `tracing::info!("mDNS discovery enabled")` call from
`DiscoveryService::start()` at `crates/periphore-discovery/src/lib.rs:99`. The caller in
`main.rs` already has appropriate context (`config.discovery.enabled` guard) and logs at
the right level. Similarly for SSH probe: `lib.rs:119` logs "SSH tunnel port probing
enabled" while `main.rs:192-196` logs the same with the ports list — prefer the richer
`main.rs` log and remove the one in `lib.rs`.

---

## Info

### IN-01: `periphore-discovery` library has `test = false` which silently drops inline unit tests

**File:** `crates/periphore-discovery/Cargo.toml:13-14`

**Issue:** `[lib]\ntest = false` disables the built-in test harness for the library
target. Any `#[test]` function added inside `src/*.rs` files will be silently ignored
rather than run. Integration tests under `tests/` are unaffected (they compile as separate
binaries with their own test harness). The intent of the setting is unclear from the file
alone — if it was added to avoid re-running integration tests as "unit tests", that
heuristic does not apply here (they live in separate directories). If it was added to
suppress a specific compilation issue, that context should be in a comment.

**Fix:** Remove `test = false` from `[lib]` unless there is a documented reason to keep
it. If the intent was to keep integration tests isolated, the default Cargo behavior
already does this:

```toml
[lib]
# test = false  <-- remove this line
```

---

### IN-02: `own_fingerprint` parameter in `ssh_probe_loop` is redundant with `identity`

**File:** `crates/periphore-discovery/src/probe.rs:41-47` and `crates/periphore-discovery/src/lib.rs:104-105`

**Issue:** `ssh_probe_loop` receives both `own_fingerprint: [u8; 32]` and
`identity: Arc<IdentityStore>`. The `own_fingerprint` value is extracted from
`identity.fingerprint` at the call site (`lib.rs:104`). The function signature carries
the same data twice via two different bindings. This is a minor code smell that could
cause drift if the call site logic is changed.

**Fix:** Remove the `own_fingerprint` parameter and read it directly from the `identity`
inside the function:

```rust
pub(crate) async fn ssh_probe_loop(
    ports: Vec<u16>,
    identity: Arc<IdentityStore>,
    peers: Arc<std::sync::Mutex<DiscoveredPeerList>>,
    event_tx: mpsc::Sender<DiscoveryEvent>,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    let own_fingerprint = identity.fingerprint;
    // ... rest unchanged
}
```

Update the call site in `lib.rs` to drop the pre-extraction.

---

### IN-03: `instance_name` falls back to the literal string "periphore" instead of the system hostname

**File:** `crates/periphore-discovery/src/lib.rs:82-84`

**Issue:** When `config.instance_name` is `None`, the mDNS instance name defaults to the
hardcoded string `"periphore"`. The field documentation in `schema.rs:146` says "Default:
system hostname (set at runtime by periphore-discovery)". The implementation does not
match the documented default — every host with `instance_name` unset will advertise the
same instance name `"periphore"`, causing mDNS naming conflicts and making it impossible
for the browsing side to distinguish between multiple peers on the same subnet.

**Fix:** Resolve the actual system hostname at runtime:

```rust
let instance_name = config.instance_name.clone().unwrap_or_else(|| {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "periphore".to_owned())
});
```

The `hostname` crate (or `std::net::hostname` equivalent) is not yet in the workspace
dependencies. Alternatively, use `gethostname` (in the `nix` crate already likely
available for Linux targets), or shell out to `hostname` — but the cleanest cross-platform
option is the `hostname` crate. If adding a dependency is not desired for this phase, at
minimum change the fallback value to something unique (e.g., incorporating a portion of
the identity fingerprint):

```rust
let instance_name = config.instance_name.clone().unwrap_or_else(|| {
    format!("periphore-{}", &identity.fingerprint_hex()[..8])
});
```

This avoids mDNS name collisions when multiple nodes use the default config.

---

### IN-04: Peer list cap eviction test relies on insertion ordering rather than time ordering

**File:** `crates/periphore-discovery/tests/integration.rs:38-62`

**Issue:** `list_cap_eviction` inserts 64 entries in a tight loop and asserts that
`host-0` (the first inserted) is the evicted entry when a 65th is inserted. This holds
because `Instant::now()` will return the same or nearly-identical timestamp for all 64
insertions in a CPU-bound loop, making the `min_by_key` comparison effectively
non-deterministic on fast machines — the evicted entry could be any of the 64 peers with
the same `last_seen` value, not necessarily `host-0`. The test passes in practice on slow
hardware or when the loop takes more than 1 nanosecond per iteration, but is inherently
flaky at the nanosecond level.

**Fix:** Insert a brief sleep between the first and subsequent entries, or use a manual
`last_seen` injection mechanism, to guarantee `host-0` has an observably older timestamp:

```rust
// Insert host-0 first with a known-old timestamp
list.upsert("host-0".to_owned(), 7888, DiscoverySource::Mdns, None);
std::thread::sleep(std::time::Duration::from_millis(5)); // ensure older last_seen

// Insert remaining 63 entries — all newer than host-0
for i in 1..64u16 {
    list.upsert(format!("host-{i}"), 7888 + i, DiscoverySource::Mdns, None);
}
// ... rest of test unchanged
```

---

_Reviewed: 2026-04-28T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
