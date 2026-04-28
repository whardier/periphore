---
phase: 6
slug: tcp-peering
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-26
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` + `#[tokio::test]` for async |
| **Config file** | None — `[lib] test = false` + integration tests in `tests/` per workspace pattern |
| **Quick run command** | `cargo test -p periphore-net -p periphored` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p periphore-net -p periphored`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 6-01-01 | 01 | 0 | NET-01 | — | N/A | integration | `cargo test -p periphore-net --test integration` | ❌ W0 | ⬜ pending |
| 6-01-02 | 01 | 0 | NET-01 | T-6-01 | Malformed frame returns Err, never panics | integration | `cargo test -p periphore-net --test integration trusted_handshake` | ❌ W0 | ⬜ pending |
| 6-01-03 | 01 | 0 | NET-01 | T-6-02 | Unknown peer placed in Pending state, not dropped | integration | `cargo test -p periphore-net --test integration pending_handshake` | ❌ W0 | ⬜ pending |
| 6-02-01 | 02 | 1 | NET-01 | T-6-03 | Protocol version mismatch → HelloAck accepted=false + disconnect | integration | `cargo test -p periphore-net --test integration version_mismatch` | ❌ W0 | ⬜ pending |
| 6-02-02 | 02 | 1 | NET-01 | T-6-04 | Fingerprint conflict (SEC-06) → tracing::error! + drop | integration | `cargo test -p periphore-net --test integration fingerprint_conflict` | ❌ W0 | ⬜ pending |
| 6-02-03 | 02 | 1 | NET-01 | T-6-05 | AcceptFingerprint promotes pending peer to connected | integration | `cargo test -p periphore-net --test integration promote_pending` | ❌ W0 | ⬜ pending |
| 6-03-01 | 03 | 1 | NET-03 | — | PeerConfig.host triggers auto-connect at startup | integration | `cargo test -p periphored --test net_wiring` | ❌ W0 | ⬜ pending |
| 6-03-02 | 03 | 1 | NET-06 | T-6-06 | macOS: non-TTY stdin prints clear error and exits | unit | `cargo test -p periphored` (cfg-gated test) | ❌ W0 | ⬜ pending |
| 6-04-01 | 04 | 2 | NET-01 | — | GetPendingVerifications returns real pending list | integration | `cargo test -p periphored --test net_wiring pending_ipc` | ❌ W0 | ⬜ pending |
| 6-04-02 | 04 | 2 | NET-04 | — | No UDP sockets in periphore-net (static grep) | static | `grep -r "UdpSocket" crates/periphore-net/src/ -- returns empty` | — | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/periphore-net/tests/integration.rs` — stubs + implementations for NET-01 (handshake, trusted, pending, version mismatch, fingerprint conflict, promote pending)
- [ ] `crates/periphored/tests/net_wiring.rs` — NET-03 auto-connect from config, GetPendingVerifications IPC wiring

*Integration test design: bind `TcpListener` on `127.0.0.1:0`, connect from second task, run handshake with fabricated `IdentityStore` + `TrustStore`. Assert `HandshakeResult::Pending` for unknown peer. Call `promote_pending()` and assert `PeerEvent::PeerConnected`. Fully in-process, no external infrastructure.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Two machines complete handshake and report "connected" status | NET-01 (SC1) | Requires two physical/VM machines with network access | Start `periphored` on both; configure `[[peer]]` with host on one side; confirm `PeerConnected` log on both |
| Connection works through SSH tunnel | NET-04 (SC2) | Requires SSH tunnel setup (`ssh -L`) | Establish `ssh -L 7888:remote:7888 user@host`; configure local `[[peer]]` pointing to `127.0.0.1:7888`; confirm handshake completes |
| Daemon stays running after SSH session ends on Linux | NET-05 (SC3) | Requires remote Linux host + SSH session | Launch via `nohup periphored &` or systemd user unit; disconnect SSH; confirm daemon is still running via second SSH session |
| macOS launchd launch (not SSH) works correctly | NET-06 (SC4) | Requires macOS with launchd configuration | Configure launchd plist; verify daemon starts without TTY error |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
