---
phase: 1
slug: workspace-protocol-foundation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-22
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test (`#[test]`, `#[cfg(test)]`) |
| **Config file** | None required — Cargo handles test discovery |
| **Quick run command** | `cargo test -p periphore-protocol && cargo clippy --workspace` |
| **Full suite command** | `cargo test --workspace && cargo build --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p periphore-protocol && cargo clippy --workspace`
- **After every plan wave:** Run `cargo test --workspace && cargo build --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** ~30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 1-01-01 | 01 | 1 | CFG-01 | — | No Serialize on Config struct (compile-time) | Compile check | `cargo build -p periphore-config` | ❌ W0 | ⬜ pending |
| 1-01-02 | 01 | 1 | CFG-01 | — | No write paths in periphore-config source | Negative compile test | `cargo build -p periphore-config` | ❌ W0 | ⬜ pending |
| 1-01-03 | 01 | 1 | IPC-01 | T-1-03 | Socket created at platform path; permissions 0600 | Integration | `cargo test -p periphore-ipc -- ipc::tests::socket_creates` | ❌ W0 | ⬜ pending |
| 1-01-04 | 01 | 1 | IPC-01 | T-1-04 | Socket removed on clean shutdown | Integration | `cargo test -p periphore-ipc -- ipc::tests::socket_removed_on_shutdown` | ❌ W0 | ⬜ pending |
| 1-01-05 | 01 | 2 | IPC-02 | T-1-05 | GetStatus returns response over socket | Integration | `cargo test -p periphore-ipc -- ipc::tests::get_status_response` | ❌ W0 | ⬜ pending |
| 1-01-06 | 01 | 2 | IPC-02 | — | InjectInputEvent accepted without network peer | Integration | `cargo test -p periphore-ipc -- ipc::tests::inject_input_no_peer` | ❌ W0 | ⬜ pending |
| 1-01-07 | 01 | 1 | — | — | PeerMessage all variants round-trip via postcard | Unit | `cargo test -p periphore-protocol -- peer::tests` | ❌ W0 | ⬜ pending |
| 1-01-08 | 01 | 1 | — | — | IpcRequest all variants round-trip via serde_json | Unit | `cargo test -p periphore-protocol -- ipc::tests` | ❌ W0 | ⬜ pending |
| 1-01-09 | 01 | 3 | — | — | cargo build --workspace succeeds | Build check | `cargo build --workspace` | ❌ W0 | ⬜ pending |
| 1-01-10 | 01 | 3 | — | — | periphore --help produces output | Smoke | `./target/debug/periphore --help` | ❌ W0 | ⬜ pending |
| 1-01-11 | 01 | 3 | — | — | periphored --help produces output | Smoke | `./target/debug/periphored --help` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/periphore-protocol/src/peer.rs` — `PeerMessage` round-trip tests (all ~15 variants)
- [ ] `crates/periphore-protocol/src/ipc.rs` — `IpcRequest`/`IpcResponse` round-trip tests
- [ ] `crates/periphore-ipc/tests/socket.rs` — IPC socket lifecycle integration tests
- [ ] `crates/periphore-config/tests/config.rs` — config layering and no-write invariant
- [ ] Root `Cargo.toml` with workspace configuration — entire workspace needs creation

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `$XDG_RUNTIME_DIR` socket path on Linux | IPC-01 | Not testable on macOS dev machine | Start daemon on Linux, verify socket at `$XDG_RUNTIME_DIR/periphore/periphore.sock` |
| `directories` crate `runtime_dir()` returns None on macOS | IPC-01 | Runtime behavior of third-party crate | Verify fallback to `$TMPDIR/periphore/periphore.sock` on macOS in integration test |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
