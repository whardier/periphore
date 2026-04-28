---
phase: 7
slug: peer-discovery
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-28
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in Rust test harness) |
| **Config file** | Per-crate `Cargo.toml` — `[lib] test = false` + `tests/` subdir |
| **Quick run command** | `cargo test -p periphore-discovery && cargo test -p periphore-config && cargo build --workspace` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p periphore-discovery && cargo test -p periphore-config && cargo build --workspace`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 07-01-01 | 01 | 1 | NET-02 | T-07-01 | mDNS bind failure logs warn, daemon continues | unit | `cargo test -p periphore-discovery -- mdns_bind_failure` | ❌ W0 | ⬜ pending |
| 07-01-02 | 01 | 1 | NET-02 | — | N/A | unit | `cargo test -p periphore-config -- discovery_config_default` | ❌ W0 | ⬜ pending |
| 07-02-01 | 02 | 1 | NET-02 | — | N/A | integration | `cargo test -p periphore-discovery -- mdns_register_browse` | ❌ W0 | ⬜ pending |
| 07-02-02 | 02 | 1 | NET-02 | T-07-02 | Peer list caps at 64; oldest evicted on overflow | unit | `cargo test -p periphore-discovery -- list_cap_eviction` | ❌ W0 | ⬜ pending |
| 07-02-03 | 02 | 1 | NET-02 | — | N/A | unit | `cargo test -p periphore-discovery -- gc_removes_expired` | ❌ W0 | ⬜ pending |
| 07-03-01 | 03 | 1 | NET-02 | T-07-03 | SSH probe skips non-Periphore services | unit | `cargo test -p periphore-discovery -- ssh_probe_non_periphore` | ❌ W0 | ⬜ pending |
| 07-03-02 | 03 | 1 | NET-02 | T-07-03 | SSH probe skips own daemon (fingerprint match) | unit | `cargo test -p periphore-discovery -- ssh_probe_self_detection` | ❌ W0 | ⬜ pending |
| 07-03-03 | 03 | 1 | NET-02 | — | N/A | integration | `cargo test -p periphore-discovery -- ssh_probe_discovers_daemon` | ❌ W0 | ⬜ pending |
| 07-04-01 | 04 | 2 | NET-02 | — | N/A | integration | `cargo test -p periphore-cli -- peers_discovered` | ❌ W0 | ⬜ pending |
| 07-04-02 | 04 | 2 | NET-02 | — | N/A | integration | `cargo test -p periphore-cli -- peers_pending` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/periphore-discovery/Cargo.toml` — new crate scaffold with `[lib] test = false`, all required deps
- [ ] `crates/periphore-discovery/src/lib.rs` — crate root stub (DiscoveryService, DiscoveryEvent)
- [ ] `crates/periphore-discovery/tests/integration.rs` — test stubs for NET-02 SC1 (mDNS register+browse), SC3 (mDNS failure), SSH probe tests
- [ ] Framework install: none needed (cargo test built-in)

*Existing infrastructure covers CLI and config tests — only `periphore-discovery` crate needs scaffolding.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Peer appears in discovered list within 5 seconds on same subnet | NET-02 SC1 | Requires two real machines on the same LAN; loopback mDNS differs from real multicast | Start daemon on machine A and B with `[discovery] enabled = true`; run `periphore peers discovered` on machine A within 5 seconds of starting B |
| mDNS silent failure on corporate/blocked network | NET-02 SC3 | Requires controlled network environment (firewall blocking 5353) | Block UDP port 5353; start daemon with discovery enabled; verify `tracing::warn!` in logs; verify `periphore peers discovered` returns empty; verify `[[peer]] host=` manual config still connects |
| SSH tunnel peer discovery with `ssh -L/-R` forwarding | NET-02-SSH | Requires live SSH tunnel setup between two machines | Set up `ssh -L 17888:localhost:7888 user@host`; enable `ssh_probe_enabled = true`; run `periphore peers discovered` and confirm forwarded peer appears |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
