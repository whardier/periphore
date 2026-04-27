---
phase: 06-tcp-peering
reviewed: 2026-04-27T00:00:00Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - crates/periphore-config/src/schema.rs
  - crates/periphore-config/tests/config.rs
  - crates/periphore-identity/src/lib.rs
  - crates/periphore-net/src/codec.rs
  - crates/periphore-net/src/connection.rs
  - crates/periphore-net/src/error.rs
  - crates/periphore-net/src/event.rs
  - crates/periphore-net/src/handshake.rs
  - crates/periphore-net/src/lib.rs
  - crates/periphore-net/src/manager.rs
  - crates/periphore-net/tests/codec_error.rs
  - crates/periphore-net/tests/event_connection.rs
  - crates/periphore-net/tests/integration.rs
  - crates/periphore-protocol/src/ipc.rs
  - crates/periphore-protocol/src/lib.rs
  - crates/periphore-protocol/tests/roundtrip.rs
  - crates/periphored/src/main.rs
  - crates/periphored/tests/net_wiring.rs
findings:
  critical: 1
  warning: 5
  info: 4
  total: 10
status: issues_found
---

# Phase 06: Code Review Report

**Reviewed:** 2026-04-27
**Depth:** standard
**Files Reviewed:** 18
**Status:** issues_found

## Summary

This phase implements the core TCP peering layer: `LengthDelimitedCodec`+postcard framing, a two-step Hello/HelloAck handshake, a `ConnectionManager` with exponential-backoff reconnect, and wiring of the network layer into `periphored`. The architecture is clean and the critical CLAUDE.md constraints (TCP_NODELAY, max_frame_length, handshake timeout) are all correctly enforced.

One critical bug was found in the pending-peer promotion path: after `ConnectionControl::PromoteTrusted` is received, the task holds the connection "open" by reading frames, but PeerDisconnected is never sent when a promoted peer's connection finally closes. The connection is silently dropped from the daemon's perspective after promotion. Five warnings cover a TOCTOU race in the integration test, a missing flush after the HelloAck in the responder's success path, peer_key disambiguation collision risk, unlocked trust-store panic on poisoning, and unsafe env mutation in tests. Four info items cover minor quality concerns.

---

## Critical Issues

### CR-01: PeerDisconnected never emitted for promoted pending peers

**File:** `crates/periphore-net/src/manager.rs:190-203`

**Issue:** In both `spawn_listener` and `spawn_connector`, when a handshake resolves to `HandshakeResult::Pending` and the user subsequently sends `ConnectionControl::PromoteTrusted`, a `PeerEvent::PeerConnected` is emitted. However, the task immediately returns after emitting `PeerConnected` — there is no "hold connection open" loop equivalent to the `Trusted` path. When the promoted peer's socket closes (immediately, because there is no read loop), no `PeerEvent::PeerDisconnected` is ever sent to the daemon. This means the daemon's `focus_sm.reclaim()` on disconnect is never triggered, and any consumer of peer connection state will permanently believe the peer is still connected.

The `Trusted` path (lines 139–155 and 303–320) correctly runs a hold-open loop followed by `PeerEvent::PeerDisconnected`. The `Pending`→promoted path (lines 190–198 and 356–364) does neither.

**Fix:** After emitting `PeerEvent::PeerConnected` following promotion, run the same hold-open loop and emit `PeerDisconnected` on exit:

```rust
Some(ConnectionControl::PromoteTrusted) => {
    event_tx
        .send(PeerEvent::PeerConnected { peer_id: peer_id.clone() })
        .await
        .ok();
    // Hold connection open until EOF/error (same as Trusted path)
    loop {
        match tokio::time::timeout(
            Duration::from_secs(30),
            framed_read.next(),
        )
        .await
        {
            Ok(Some(Ok(_frame))) => {}
            _ => break,
        }
    }
    event_tx
        .send(PeerEvent::PeerDisconnected { peer_id })
        .await
        .ok();
}
```

Both the inbound (listener) and outbound (connector) pending arms require this fix.

---

## Warnings

### WR-01: TOCTOU race in integration test `promote_pending`

**File:** `crates/periphore-net/tests/integration.rs:407-419`

**Issue:** The `promote_pending` test pre-binds a `TcpListener` on port 0 to discover a free port, then `drop`s it and calls `conn_mgr.spawn_listener()` with the same address. The comment acknowledges "TOCTOU window acceptable in tests," but the window is real: another process or test can claim that port between the `drop` and the bind inside `spawn_listener`. This can cause the test to fail non-deterministically in CI, particularly on Linux where port reuse happens quickly. This pattern is also used in `crates/periphored/tests/net_wiring.rs` indirectly.

The test already has a `tokio::time::sleep(50ms)` to wait for the bind — the real problem is the TOCTOU, not the timing.

**Fix:** Pass a `TcpListener` directly into `spawn_listener` so the test binds once and transfers the already-bound socket:

```rust
// Option 1: Change spawn_listener to accept an already-bound TcpListener
pub fn spawn_listener(
    &mut self,
    tasks: &mut JoinSet<anyhow::Result<()>>,
    listener: TcpListener,   // <-- pass pre-bound socket
    identity: Arc<IdentityStore>,
    trust_store: Arc<RwLock<TrustStore>>,
) { ... }
```

If the API cannot change for this phase, at minimum add a retry loop in the test rather than a fixed sleep after the drop.

### WR-02: Missing flush after successful HelloAck in responder

**File:** `crates/periphore-net/src/handshake.rs:232-235`

**Issue:** After sending `HelloAck { accepted: true }` (the success path, line 227–234), `framed_write.flush()` is never called. The rejection paths at lines 193–194 and 219–220 both correctly call `flush()` after send. For the success case, the bytes may remain in the write buffer and not be transmitted until the next write. With `TCP_NODELAY` set this is unlikely to cause visible latency in practice, but the inconsistency is a correctness concern: if the initiator is waiting on the HelloAck before sending further data, and no further write from the responder occurs to flush the codec's internal buffer, the handshake can hang.

**Fix:**
```rust
framed_write
    .send(encode_message(&ack)?)
    .await
    .map_err(NetError::Io)?;
framed_write.flush().await.map_err(NetError::Io)?;  // add this
```

### WR-03: `peer_key` collision between name and host

**File:** `crates/periphore-net/src/manager.rs:244-250`

**Issue:** The key stored in `peer_tokens` for a connector task is derived as `peer_config.name` if set, otherwise `peer_config.host.unwrap_or_default()`. The SIGHUP diff logic in `periphored/src/main.rs` (lines 205–215 and 324–334) constructs a different key format: `format!("{}:{}", h, p.port.unwrap_or(DEFAULT_PORT))`. These two key schemes are incompatible: `cancel_peer()` is called with a `"host:port"` string (e.g., `"192.168.1.100:7888"`), but `peer_tokens` may be keyed by name (e.g., `"work-mac"`) or by host only (e.g., `"192.168.1.100"`). The result is that `cancel_peer()` will never find the token and the connector task will keep reconnecting after a peer is removed from config.

**Fix:** Either unify the key scheme in both `spawn_connector` and the SIGHUP/ReloadConfig diff logic to the same format, or expose the key from `spawn_connector` so callers use it directly:

```rust
// Return the actual key used so callers can cancel with the right value
pub fn spawn_connector(
    &mut self,
    tasks: &mut JoinSet<anyhow::Result<()>>,
    peer_config: PeerConfig,
    identity: Arc<IdentityStore>,
    trust_store: Arc<RwLock<TrustStore>>,
) -> String {  // return the key
    let peer_key = peer_config
        .name
        .clone()
        .unwrap_or_else(|| peer_config.host.clone().unwrap_or_default());
    // ... same key stored in peer_tokens ...
    peer_key  // caller uses this for cancel_peer()
}
```

### WR-04: Trust-store lock poisoning converted to wrong error variant

**File:** `crates/periphore-net/src/handshake.rs:114-117` and `crates/periphore-net/src/handshake.rs:238-241`

**Issue:** When `trust_store.read()` returns `Err` (poison), the error is converted to `NetError::Decode("trust lock poisoned")`. This is semantically incorrect — a lock-poison error is not a decode error. More importantly, this silently accepts any poisoned lock (which indicates a panic in another thread) by mapping it to a non-fatal protocol error string. A poisoned `RwLock` means the system is in an undefined state and the connection should be rejected with a distinct error.

**Fix:** Add a `NetError::Internal(String)` variant, or at minimum use `NetError::ConnectionClosed` to abort the connection rather than surfacing a misleading decode error:

```rust
// In error.rs, add:
#[error("internal error: {0}")]
Internal(String),

// In handshake.rs:
.read()
.map_err(|_| NetError::Internal("trust store lock poisoned".into()))?
.is_trusted(&peer_fp_hex);
```

### WR-05: Unsafe `std::env::set_var` / `remove_var` in tests without documented thread-safety justification

**File:** `crates/periphore-config/tests/config.rs:23-24` and `64-66`

**Issue:** `std::env::set_var` and `std::env::remove_var` are marked `unsafe` in Rust 2024 (stabilized in 1.79+) because modifying environment variables is not safe in a multi-threaded process. The code uses a `static ENV_MUTEX: Mutex<()>` and acquires it before every mutation, which is the correct approach for serializing env access within the test binary. However, the comment says "Safety: ... the ENV_MUTEX ensures no concurrent test is reading config while we mutate env state." This is only true if all tests in the binary that touch env vars acquire the same mutex, and if no background threads (e.g., spawned by `figment` or `tempfile`) read env vars concurrently. The mutex does not protect against threads that do not know to acquire it.

This is a warning rather than critical because the existing approach is standard test practice and the mutex does cover the test-authored paths. The concern is correctness if the test binary is expanded.

**Fix:** Add a comment documenting the scope of the mutex guarantee and the assumption that no third-party code in the test process reads `PERIPHORE_*` env vars on background threads:

```rust
// SAFETY: ENV_MUTEX serializes all PERIPHORE_* env var mutations in this
// test binary. This assumes no background thread (e.g., from figment,
// tempfile, or tokio) reads PERIPHORE_* vars concurrently. If that
// assumption breaks, move to process-isolated tests (separate test binary
// per env-sensitive test).
unsafe { std::env::remove_var("PERIPHORE_LOGGING_LEVEL") };
```

---

## Info

### IN-01: `build_border` panics on label longer than 13 characters

**File:** `crates/periphore-identity/src/lib.rs:236`

**Issue:** `build_border` computes `let dash_count = 13 - label.len()`. If `label.len() > 13`, this underflows (subtraction on `usize` panics in debug mode, wraps in release). The function is only called with two hardcoded labels (`"ED25519 256"`, 11 chars, and `"PERIPHORE"`, 9 chars), so this is not currently reachable. However, the function is not marked `pub`, and there is no assertion guarding the arithmetic.

**Fix:** Add a saturating subtraction or a debug assert:
```rust
let dash_count = 13usize.saturating_sub(label.len());
```

### IN-02: `word_indices` can panic on short fingerprints via free function

**File:** `crates/periphore-identity/src/lib.rs:253-258`

**Issue:** `word_indices` indexes `fingerprint[byte_offset + 2]`. For `i = 5`, `bit_offset = 55`, `byte_offset = 6`, so the maximum index is `byte_offset + 2 = 8`. Since the fingerprint is `[u8; 32]` this is always in bounds. But the concern is that `word_phrase_from_fingerprint` and `identicon_from_fingerprint` accept `&[u8; 32]`, which is type-safe by construction. This is an info item only: the type signature prevents misuse, but there are no explicit bounds comments to aid future maintainers who might refactor to slices.

**Fix:** Add a comment or a `debug_assert_eq!(fingerprint.len(), 32)` in `word_indices` to document the invariant.

### IN-03: `PeerEvent` and `HandshakeResult` do not derive `PartialEq`

**File:** `crates/periphore-net/src/event.rs` and `crates/periphore-net/src/connection.rs`

**Issue:** `PeerEvent` and `HandshakeResult` use `matches!()` in tests (e.g., `matches!(init_result, Ok(HandshakeResult::Trusted { .. }))`), which works. But as these types grow, the absence of `PartialEq` will prevent writing direct `assert_eq!` tests on them, requiring workarounds. `ActiveConn`, `PendingPeer`, and `ConnectionControl` also lack `PartialEq`. Adding `PartialEq` now costs nothing and avoids future friction.

**Fix:** Add `PartialEq` to the derives where `mpsc::Sender<ConnectionControl>` does not block it (note: `mpsc::Sender` does implement `PartialEq` in Tokio).

### IN-04: Connector defaults to `127.0.0.1` when host is `None`

**File:** `crates/periphore-net/src/manager.rs:258-261`

**Issue:** When `spawn_connector` is called with a `PeerConfig` whose `host` is `None`, the connector silently defaults to `"127.0.0.1"`. `periphored/src/main.rs` already guards against this with `if peer.host.is_some()` (line 161), but the fallback inside `spawn_connector` can still fire if called directly (e.g., in tests or future callers). A peer config with no host that causes a loopback connection is unlikely to be the intended behavior.

**Fix:** Return an error or panic-with-message if `peer_config.host` is `None`:
```rust
let host = peer_config
    .host
    .as_deref()
    .ok_or_else(|| anyhow::anyhow!("peer config has no host — cannot connect"))?
    .to_owned();
```

---

_Reviewed: 2026-04-27_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
