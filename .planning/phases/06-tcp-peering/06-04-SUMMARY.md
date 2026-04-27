---
phase: "06"
plan: "06-04"
subsystem: tcp-peering
tags: [periphored, connection-manager, focus-state-machine, systemd, macos-ssh-guard, config-reload, peer-diff]
dependency_graph:
  requires:
    - "06-01 (periphore-net Cargo.toml, DaemonConfig.listen)"
    - "06-02 (NetError, codec, PeerEvent, connection types)"
    - "06-03 (handshake.rs, manager.rs, lib.rs -- periphore-net complete)"
  provides:
    - periphored wired as a real TCP peer daemon
    - macOS SSH detection guard (D-15, D-16, NET-06)
    - ConnectionManager initialized and driving listener/connectors
    - net_event_rx arm in main select! loop (PeerPending WARN, PeerDisconnected reclaim)
    - AcceptFingerprint promotes pending connections via promote_pending()
    - GetPendingVerifications returns real pending list (D-03)
    - SimulateEdgeCross logs FocusStateMachine state (D-21)
    - D-11 peer list diff on both SIGHUP and ReloadConfig arms
    - daemon.listen in reload_config restart-required check
    - contrib/periphored.service systemd user unit (NET-05, D-14)
  affects:
    - crates/periphored/Cargo.toml
    - crates/periphored/src/main.rs
    - crates/periphore-config/src/schema.rs
    - contrib/periphored.service
tech_stack:
  added:
    - periphore-net workspace dep in periphored
    - periphore-core workspace dep in periphored
    - Clone derive on PeerConfig (required for spawn_connector by value)
  patterns:
    - Arc<IdentityStore> wrapping identity for multi-task sharing
    - Arc<RwLock<TrustStore>> wrapping trust_store for concurrent read/write
    - HashSet diff pattern for D-11 peer list cancellation on both reload paths
    - macOS cfg-gated IsTerminal check before any async setup
    - tokio::sync::mpsc channel for PeerEvent routing from ConnectionManager to daemon loop
key_files:
  created:
    - contrib/periphored.service
  modified:
    - crates/periphored/Cargo.toml
    - crates/periphored/src/main.rs
    - crates/periphore-config/src/schema.rs
key_decisions:
  - "identity and trust_store wrapped in Arc<_> in periphored -- required for ConnectionManager sharing without double-loading"
  - "Clone derive added to PeerConfig in schema.rs -- spawn_connector takes PeerConfig by value; for-loop borrows required .clone()"
  - "D-11 diff duplicated in both SIGHUP arm and ReloadConfig IPC arm (not extracted to helper) -- conn_mgr.cancel_peer() takes &mut self, conn_mgr must be in scope; both select! arms have direct access"
  - "GetPendingVerifications removed from send_ok() match and promoted to dedicated select! arm -- compiler enforced via wildcard _ catch-all in send_ok()"

requirements-completed:
  - NET-03
  - NET-05
  - NET-06

duration: 3min
completed: "2026-04-27"
---

# Phase 6 Plan 4: periphored wiring + macOS SSH check + systemd unit Summary

**periphored wired as a real TCP peer daemon: ConnectionManager + FocusStateMachine initialized, listener/connectors spawned from config, net events dispatched in select! loop, AcceptFingerprint promotes pending peers, D-11 peer list diff cancels orphaned reconnect tasks on both reload paths, macOS SSH guard added, systemd unit delivered.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-04-27T09:14:42Z
- **Completed:** 2026-04-27T09:17:37Z
- **Tasks:** 2
- **Files modified:** 4 (3 modified, 1 created)

## Accomplishments

- Added `periphore-net` and `periphore-core` workspace deps to `periphored/Cargo.toml`
- Added `Clone` derive to `PeerConfig` in `periphore-config/src/schema.rs` (required for `spawn_connector` by value)
- Added macOS SSH detection guard (`#[cfg(target_os = "macos")] { if !stdin().is_terminal() { exit(1) } }`) at top of `main()` before any async setup — satisfies D-15, D-16, NET-06, T-6-06
- Wrapped `identity` in `Arc<IdentityStore>` and `trust_store` in `Arc<RwLock<TrustStore>>` for multi-task sharing with connection tasks
- Initialized `ConnectionManager::new()` and `FocusStateMachine::new()` in daemon startup
- Spawned TCP listener (`spawn_listener`) when `config.daemon.listen = true` (D-07); spawned outbound connectors (`spawn_connector`) for all peers with `host` set (D-05)
- Added `net_event_rx.recv()` arm in `select!` loop: `PeerPending` logs at WARN with fingerprint (D-02); `PeerDisconnected` calls `focus_sm.reclaim()` to return local focus
- `AcceptFingerprint` now calls `trust_store.write().unwrap().add_trusted()` then `conn_mgr.promote_pending()` — couples trust store write with network state promotion (T-6-04)
- `GetPendingVerifications` promoted from `send_ok()` stub to dedicated `select!` arm returning `conn_mgr.pending_list()` as `IpcResponse::PendingPeers` (D-03)
- `SimulateEdgeCross` updated to log `focus_sm.current_state()` (D-21)
- `daemon.listen` added to `reload_config` restart-required block
- D-11 peer list diff (HashSet old vs new, `cancel_peer()` for each removed key) wired into both SIGHUP arm and ReloadConfig IPC arm
- Created `contrib/periphored.service` systemd user unit with `Type=simple`, `ExecStart=%h/.cargo/bin/periphored`, `Restart=on-failure`, `NoNewPrivileges=true`, install instructions including `loginctl enable-linger` (NET-05, D-13, D-14)
- `cargo build --workspace` exits 0; `cargo test --workspace` all 83 tests pass

## Task Commits

1. **Task 1: periphored Cargo.toml + main.rs Phase 6 wiring** -- `8772906` (feat)
2. **Task 2: contrib/periphored.service systemd unit** -- `9df7843` (feat)

## Files Created/Modified

- `crates/periphored/Cargo.toml` -- added periphore-net and periphore-core workspace deps
- `crates/periphore-config/src/schema.rs` -- added Clone derive to PeerConfig
- `crates/periphored/src/main.rs` -- full Phase 6 daemon wiring (macOS guard, ConnectionManager, FocusStateMachine, listener/connectors, net_event arm, AcceptFingerprint promote, GetPendingVerifications real dispatch, D-11 peer diff, daemon.listen restart check)
- `contrib/periphored.service` -- systemd user unit with install instructions

## Decisions Made

- `identity` wrapped as `Arc<IdentityStore>` to share with `spawn_listener`/`spawn_connector` without a second file load. The original plan suggested loading identity twice; this avoids the double I/O.
- `Clone` derive added to `PeerConfig` as a Rule 2 (critical correctness) fix — `spawn_connector` takes `PeerConfig` by value and the `for peer in &config.peers` loop borrows; without `Clone` the code would not compile.
- D-11 diff code duplicated in both reload arms rather than extracted to a helper function — `cancel_peer()` takes `&mut self` and `conn_mgr` must be in the calling scope. A helper would need `&mut ConnectionManager` and `&Config` parameters but would reduce readability of the select! arms.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical Functionality] Added Clone derive to PeerConfig**
- **Found during:** Task 1 (build)
- **Issue:** Plan action 2d uses `peer.clone()` to pass a `PeerConfig` by value to `spawn_connector`, but `PeerConfig` only derived `Debug, Deserialize, Default` — not `Clone`. This would be a compile error.
- **Fix:** Added `Clone` to the derive list in `crates/periphore-config/src/schema.rs`
- **Files modified:** `crates/periphore-config/src/schema.rs`
- **Committed in:** `8772906` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (missing critical functionality)
**Impact on plan:** The plan explicitly noted "PeerConfig Clone requirement" in the action text and listed `schema.rs` as a file requiring editing. The fix followed the plan's own instructions precisely.

## Known Stubs

- `focus_sm.reclaim()` is called on `PeerDisconnected` but `focus_sm.transfer_to()` is never called from the current event loop — Phase 8 wires real topology routing and will call `transfer_to()` on edge-cross events.
- `SimulateEdgeCross` logs state but does not advance the state machine — Phase 8 adds topology routing.
- `active: HashMap<String, ActiveConn>` in `ConnectionManager` remains unwritten — Phase 9 adds input forwarding channel.

## Threat Surface Scan

| Mitigated | File | Description |
|-----------|------|-------------|
| T-6-06 | crates/periphored/src/main.rs | `#[cfg(target_os = "macos")] { if !stdin().is_terminal() { exit(1) } }` at top of `main()` before any async setup. Linux unaffected. |
| T-6-02 | crates/periphored/src/main.rs | `GetPendingVerifications` returns data only; `AcceptFingerprint` is the only promotion path, requiring explicit user action. No auto-promotion. |
| T-6-04 | crates/periphored/src/main.rs | `AcceptFingerprint` first writes to trust store (persistent), then calls `promote_pending()` (best-effort). Trust store is the authority. |

## Self-Check: PASSED

- `crates/periphored/Cargo.toml`: `periphore-net = { workspace = true }` -- FOUND
- `crates/periphored/Cargo.toml`: `periphore-core = { workspace = true }` -- FOUND
- `crates/periphore-config/src/schema.rs`: `#[derive(Debug, Clone, Deserialize, Default)]` on PeerConfig -- FOUND
- `crates/periphored/src/main.rs`: `is_terminal` -- FOUND
- `crates/periphored/src/main.rs`: `cfg(target_os = "macos")` -- FOUND
- `crates/periphored/src/main.rs`: `ConnectionManager` (3 occurrences: new, spawn_listener, spawn_connector) -- FOUND
- `crates/periphored/src/main.rs`: `FocusStateMachine` -- FOUND
- `crates/periphored/src/main.rs`: `spawn_listener` -- FOUND
- `crates/periphored/src/main.rs`: `spawn_connector` -- FOUND
- `crates/periphored/src/main.rs`: `PeerPending` -- FOUND
- `crates/periphored/src/main.rs`: `promote_pending` -- FOUND
- `crates/periphored/src/main.rs`: `pending_list` -- FOUND
- `crates/periphored/src/main.rs`: `PendingPeers` -- FOUND
- `crates/periphored/src/main.rs`: `daemon.listen` -- FOUND (3 occurrences)
- `crates/periphored/src/main.rs`: `cancel_peer(` -- FOUND (2 occurrences: SIGHUP arm, ReloadConfig arm)
- `crates/periphored/src/main.rs`: `peer removed from config` -- FOUND (2 occurrences)
- `contrib/periphored.service`: `Type=simple` -- FOUND
- `contrib/periphored.service`: `WantedBy=default.target` -- FOUND
- `contrib/periphored.service`: `Restart=on-failure` -- FOUND
- `contrib/periphored.service`: `ExecStart=%h/.cargo/bin/periphored` -- FOUND
- `contrib/periphored.service`: `NoNewPrivileges=true` -- FOUND
- `contrib/periphored.service`: `loginctl enable-linger` -- FOUND
- Commit `8772906`: FOUND
- Commit `9df7843`: FOUND
- `cargo build --workspace`: exits 0
- `cargo test --workspace`: all tests pass
