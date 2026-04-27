---
phase: 06-tcp-peering
fixed_at: 2026-04-27T00:00:00Z
review_path: .planning/phases/06-tcp-peering/06-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 06: Code Review Fix Report

**Fixed at:** 2026-04-27
**Source review:** .planning/phases/06-tcp-peering/06-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6 (1 Critical, 5 Warning)
- Fixed: 6
- Skipped: 0

## Fixed Issues

### CR-01: PeerDisconnected never emitted for promoted pending peers

**Files modified:** `crates/periphore-net/src/manager.rs`
**Commit:** 1d77198
**Applied fix:** Added a hold-open `loop` (identical to the Trusted path) and a trailing `PeerEvent::PeerDisconnected` send to the `PromoteTrusted` arm in both `spawn_listener` (inbound) and `spawn_connector` (outbound). Both arms now clone `peer_id` before the loop and emit `PeerDisconnected` on loop exit, ensuring `focus_sm.reclaim()` in periphored is triggered when the promoted peer's socket closes.

### WR-01: TOCTOU race in integration test `promote_pending`

**Files modified:** `crates/periphore-net/tests/integration.rs`
**Commit:** 898c7fe
**Applied fix:** Replaced the fixed 50 ms `tokio::time::sleep` after `drop(tmp_listener)` with a probe-connect retry loop (20 attempts × 10 ms = 200 ms max). The loop exits as soon as a TCP connection to `bound_addr` succeeds, confirming `spawn_listener`'s internal bind is ready. This eliminates the timing dependency without changing the `spawn_listener` API.

### WR-02: Missing flush after successful HelloAck in responder

**Files modified:** `crates/periphore-net/src/handshake.rs`
**Commit:** c8b828a
**Applied fix:** Added `framed_write.flush().await.map_err(NetError::Io)?` immediately after the `framed_write.send(encode_message(&ack)?)` call in the `accepted: true` success path of `perform_handshake_responder`. The rejection paths already called flush; the success path now matches them.

### WR-03: `peer_key` collision between name and host

**Files modified:** `crates/periphore-net/src/manager.rs`
**Commit:** a1610ba
**Applied fix:** Changed the fallback key in `spawn_connector` from `peer_config.host.clone().unwrap_or_default()` (host only) to `format!("{host}:{port}")` (host:port). This matches the `format!("{}:{}", h, p.port.unwrap_or(DEFAULT_PORT))` format used in the SIGHUP and ReloadConfig diff logic in `periphored/src/main.rs`, so `cancel_peer()` can now find the token for unnamed peers. Updated the doc comment to document the key format.

### WR-04: Trust-store lock poisoning converted to wrong error variant

**Files modified:** `crates/periphore-net/src/error.rs`, `crates/periphore-net/src/handshake.rs`
**Commit:** 89ea370
**Applied fix:** Added `NetError::Internal(String)` variant to `error.rs`. Replaced both occurrences of `.map_err(|_| NetError::Decode("trust lock poisoned".into()))` in `handshake.rs` (initiator and responder) with `.map_err(|_| NetError::Internal("trust store lock poisoned".into()))`. A poisoned `RwLock` is now correctly classified as an internal error rather than a protocol decode error.

### WR-05: Unsafe `std::env::set_var` / `remove_var` in tests without documented thread-safety justification

**Files modified:** `crates/periphore-config/tests/config.rs`
**Commit:** 8f301bc
**Applied fix:** Replaced the vague `// Safety: these are test-only env var mutations; the ENV_MUTEX ensures...` comment in `clear_periphore_env()` with a precise `// SAFETY:` block documenting the scope of the mutex guarantee, what it assumes about background threads, and the remediation path (process-isolated tests) if that assumption breaks. Added a matching `// SAFETY: ENV_MUTEX held; see clear_periphore_env() for full safety rationale.` comment at the inline `set_var`/`remove_var` call site in `env_overrides_toml_file`.

---

_Fixed: 2026-04-27_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
