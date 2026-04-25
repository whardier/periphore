---
phase: 4
slug: ipc-layer
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-25
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in test harness) |
| **Config file** | None — `[lib] test = false` pattern with `tests/` subdir for periphore-core |
| **Quick run command** | `cargo test -p periphore-core && cargo build -p periphored` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo build -p periphored` (or `cargo test -p periphore-core` for core tasks)
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** ~15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|--------|
| 4-01-01 | 01 | 0 | IPC-01 | T-4-01 | JoinSet guard prevents CPU spin; clean exit triggers shutdown | build | `cargo build -p periphored` | ⬜ pending |
| 4-02-01 | 02 | 1 | IPC-01 | — | Subscriber restructured with reload::Layer; filter_handle available | build | `cargo build -p periphored` | ⬜ pending |
| 4-02-02 | 02 | 1 | IPC-02 | T-4-02 | SIGHUP reloads config; ReloadConfig returns Ok/Error | build | `cargo build --workspace` | ⬜ pending |
| 4-03-01 | 03 | 1 | IPC-01 | — | periphore-core Cargo.toml is zero-dep | build | `cargo build -p periphore-core` | ⬜ pending |
| 4-03-02 | 03 | 1 | IPC-01 | — | FocusStateMachine transitions are correct | unit | `cargo test -p periphore-core --test state_machine` | ⬜ pending |
| 4-03-03 | 03 | 1 | IPC-01 | — | All 12 integration tests pass | integration | `cargo test -p periphore-core --test state_machine` | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Phase 4 has no new test infrastructure to scaffold (periphore-core tests are written in Plan 03
as part of the implementation, not a separate wave). Wave 0 is satisfied by the existing test
suite (all prior tests must remain passing before each plan executes).

*No new test stubs are required before Plan 01 — the first plan (CR-01 fix) is a targeted edit
with no new test files.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| SIGHUP actually triggers config reload in a running daemon | IPC-02 | Requires a running process + kill -HUP | Start `periphored`, modify config file, send `kill -HUP <pid>`, check logs for "config reloaded successfully" |
| CPU spin eliminated (CR-01) | IPC-01 | Requires process monitoring after IPC client disconnect | Start `periphored`, connect and disconnect a client, run `top` / `htop` — CPU should drop to 0% |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify commands
- [x] Sampling continuity: every task has an automated verify
- [x] Wave 0 covered (no missing test infrastructure — existing suite is sufficient)
- [x] No watch-mode flags
- [x] Feedback latency < 15s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending execution
