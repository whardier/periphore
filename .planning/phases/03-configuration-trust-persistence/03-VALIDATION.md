---
phase: 3
slug: configuration-trust-persistence
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-24
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in test harness) |
| **Config file** | None — cargo test is built-in; `[lib] test = false` pattern with `tests/` subdir |
| **Quick run command** | `cargo test -p periphore-trust && cargo test -p periphore-config` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p periphore-trust && cargo test -p periphore-config`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** ~10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 3-01-01 | 01 | 1 | SEC-05 | T-3-01 | Cache stored with 0o600 permissions; corrupt file returns error (not silent accept) | integration | `cargo test -p periphore-trust --test trust -- test_add_trusted_persists_across_reload` | ❌ W0 | ⬜ pending |
| 3-01-02 | 01 | 1 | SEC-05 | T-3-02 | Atomic write via tempfile+persist eliminates partial-write window | integration | `cargo test -p periphore-trust --test trust -- test_corrupt_cache_returns_error` | ❌ W0 | ⬜ pending |
| 3-01-03 | 01 | 1 | SEC-06 | T-3-03 | Fingerprint case-normalized before comparison; conflict detected on mismatch | unit | `cargo test -p periphore-trust --test trust -- test_fingerprint_conflict_detected test_fingerprint_case_insensitive` | ❌ W0 | ⬜ pending |
| 3-02-01 | 02 | 1 | CFG-02 | — | PeerConfig.name parses from TOML without breaking existing fields | integration | `cargo test -p periphore-config --test config -- test_peer_name_field` | ❌ W0 | ⬜ pending |
| 3-02-02 | 02 | 1 | CFG-03 | — | MonitorConfig entries parse from `[[topology.monitor]]` array table | integration | `cargo test -p periphore-config --test config -- test_topology_monitor_config test_topology_monitors_default_empty` | ❌ W0 | ⬜ pending |
| 3-03-01 | 03 | 2 | SEC-05 | T-3-01 | AcceptFingerprint IPC writes to trust cache; RejectFingerprint is stateless | integration | `cargo test -p periphored -- test_accept_fingerprint_writes_cache` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/periphore-trust/tests/trust.rs` — stubs for SEC-05, SEC-06 (load, add, persist, conflict, case-sensitivity, corrupt)
- [ ] `crates/periphore-config/tests/config.rs` — new test cases for CFG-02 (peer name) and CFG-03 (topology monitors); append to existing file

*Existing `cargo test` infrastructure covers all phase requirements — no new framework install needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| trusted.toml file has 0o600 permissions after first AcceptFingerprint | SEC-05 | File permission check requires OS stat; no easy Rust cross-platform test assertion | Run daemon, send AcceptFingerprint IPC, then `ls -la ~/.local/share/periphore/trusted.toml` (Linux) or `ls -la ~/Library/Application\ Support/periphore/trusted.toml` (macOS) — confirm `-rw-------` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
