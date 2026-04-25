---
phase: 5
slug: cli-tool
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-25
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) + tokio::test for async |
| **Config file** | None (Rust built-in) |
| **Quick run command** | `cargo test -p periphore-cli` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p periphore-cli`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 5-01-01 | 01 | 0 | TOP-04 / SC1 / SC3 | T-5-01 | serde_json returns Err on tampered response; never panics | integration | `cargo test -p periphore-cli` | ❌ W0 | ⬜ pending |
| 5-01-02 | 01 | 1 | SC1 | — | N/A | integration | `cargo test -p periphore-cli -- status` | ❌ W0 | ⬜ pending |
| 5-01-03 | 01 | 1 | SC3 | T-5-02 | ENOENT/ECONNREFUSED mapped to friendly "daemon not running" error | integration | `cargo test -p periphore-cli -- no_daemon` | ❌ W0 | ⬜ pending |
| 5-01-04 | 01 | 1 | TOP-04 | — | Ok response displays stub message, not error | integration | `cargo test -p periphore-cli -- topology` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/periphore-cli/tests/cli.rs` — stubs for TOP-04, SC1, SC3 (mock socket pattern from periphore-ipc/tests/socket.rs)

*Existing infrastructure covers Rust testing; only the test file and subject code are new.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `periphore status` prints fingerprint matching periphored output | SC1 | Requires a live running daemon with known identity | Start `periphored`, run `periphore status`, compare fingerprint hex |
| `periphore topology` prints stub message on real daemon | TOP-04 | Requires a live running daemon | Start `periphored`, run `periphore topology`, confirm "not yet available" message |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
