---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
current_phase: 6
current_plan: 1 (06-01 complete — protocol/config/net-crate foundations)
status: executing
stopped_at: "Completed 06-01-PLAN.md (PendingPeerInfo, IpcResponse::PendingPeers, daemon.listen, periphore-net deps)"
last_updated: "2026-04-27T08:58:32Z"
progress:
  total_phases: 10
  completed_phases: 5
  total_plans: 25
  completed_plans: 22
  percent: 88
---

# Project State

**Project:** Periphore
**Milestone:** 1 -- v1 Core
**Current phase:** 6
**Current plan:** 1 (06-01 complete — protocol/config/net-crate foundations)
**Status:** Phase 6 in progress — plan 01 complete
**Last updated:** 2026-04-27

---

## Project Reference

**Core value:** A machine's input devices should be able to reach any peer on the network, flowing naturally across screen edges, with verified identity and no central authority.

**Current focus:** Phase 06 -- TCP Peering (periphore-net)

---

## Current Position

Phase: 02 (Identity & Cryptography) -- COMPLETE
Phase: 03 (Configuration & Trust Persistence) -- COMPLETE
Phase: 04 (IPC Layer) -- COMPLETE
Phase: 05 (CLI Tool) -- COMPLETE
**Phase:** 6 of 10 (in progress — 1/5 plans complete)
**Progress:** [████████--] 88%

---

## Performance Metrics

| Metric | Value |
|--------|-------|
| Phases complete | 3/10 |
| Plans complete | 14/18 (phases 1+2+3) |
| Requirements delivered | SEC-01, SEC-02, SEC-03, SEC-04, SEC-05, SEC-06, CFG-01, CFG-02, CFG-03, IPC-01, IPC-02 (11/30) |
| Session count | 6 |

---
| Phase 04-ipc-layer P03 | 5min | 3 tasks | 3 files |
| Phase 05-cli-tool P01 | 2 | 3 tasks | 3 files |
| Phase 05-cli-tool P02 | 145 | 2 tasks | 6 files |
| Phase 05-cli-tool P03 | 1 | 1 tasks | 1 files |
| Phase 06-tcp-peering P01 | 15 | 2 tasks | 8 files |

## Accumulated Context

### Key Decisions

- Cargo workspace architecture with 12 crates: periphore-protocol, periphore-config, periphore-identity, periphore-trust, periphore-core, periphore-ipc, periphore-cli (library), periphore-net, periphore-capture, periphore-inject, periphore (CLI binary entry), periphored (daemon binary entry)
- Build order follows dependency chain: protocol -> config+identity+trust -> core+ipc+cli -> net -> capture+inject
- TCP-only transport for SSH tunnelability
- Captive window before seamless accessibility-based input
- Config never auto-writes; all config is user-authored
- Clippy pedantic group requires priority=-1 for individual lint overrides (Rust 1.95.0 lint_groups_priority)
- Cargo.lock committed since workspace produces binary crates
- All periphore-protocol tests in tests/roundtrip.rs (integration test) because [lib] test=false prevents inline test modules
- IpcRequest/IpcResponse use serde tag="type" with rename_all="snake_case" for JSON-lines IPC protocol
- Config defaults via #[serde(default)] + Default impls instead of Figment Serialized::defaults (avoids Serialize on Config, preserves CFG-01)
- ENV_MUTEX in config tests serializes access to PERIPHORE_* env vars for thread-safe parallel testing
- IpcCommand uses oneshot responder pattern: each variant carries oneshot::Sender<IpcResponse> for bidirectional IPC over Unix socket
- Each IPC integration test uses unique temp socket path with PID suffix for parallel-safe test isolation
- tokio::select! macro does not support #[cfg(unix)] on arms; guards placed on signal variable declarations instead
- periphored uses exhaustive send_ok() helper for IpcCommand dispatch with compiler-enforced variant coverage
- periphore-protocol added as direct dependency of periphored for IpcResponse type access
- periphore main.rs uses eprintln! for stub messages to keep stdout clean for future structured output
- periphore-cli uses anyhow (not thiserror) because its sole consumer is the periphore binary entry point
- rand_core 0.6 used directly (not rand 0.8/0.9) to avoid version conflict with ed25519-dalek 2.2 rand_core feature gate
- OpenOptionsExt::mode(0o600) with create_new(true) for atomic key file creation — eliminates world-readable race window
- Debug derive added to IdentityStore — required for Result<IdentityStore, IdentityError> in test panic messages
- resolve_identicon() extracted as pure free function in periphored/src/main.rs — testable without the async daemon
- tempfile promoted to workspace dep (was periphore-identity dev-dep only) — used by periphore-trust and periphored
- Drunken Bishop output is character-for-character identical on macOS (darwin 25.4.0) and Linux (rust:1-slim) — ROADMAP SC3 verified
- TrustStore uses atomic write via tempfile::NamedTempFile::new_in(parent) + persist() rename + sync_all() before rename — prevents partial writes
- Trust cache fingerprints normalized to ASCII lowercase before storage and comparison — case-insensitive by design
- TrustStore::add_trusted is idempotent (returns Ok on duplicate, updates alias) — better UX than AlreadyTrusted error
- Config.peers field requires #[serde(rename = "peer")] — TOML [[peer]] uses singular key, Rust field is plural
- PeerConfig.name is local-only label (NOT sent over wire, NOT used in identity verification) — D-11 constraint
- AcceptFingerprint IPC promotes from send_ok stub to dedicated select arm with real trust_store.add_trusted(); RejectFingerprint is stateless
- toml 0.8 with "display" feature used for trust cache serialization; features = ["display"] required for toml::to_string_pretty
- CR-01 fix: tokio::select! join_next() branch must use `, if !tasks.is_empty()` precondition guard — empty JoinSet returns Poll::Ready(None) immediately, spinning the loop at 100% CPU without the guard
- CR-01 fix: Some(Ok(Ok(()))) clean-exit arm on JoinSet must break the daemon loop — clean IPC server exit should shut down the daemon, not leave it alive with no tasks
- Config reload uses tracing_subscriber::reload::Layer wrapping EnvFilter — filter_handle stored in main() scope enables hot log-level updates without reinitializing the global subscriber
- reload_config<S: tracing::Subscriber> free function pattern avoids inline duplication while satisfying Rust's generic Handle type requirements
- daemon.socket_path and daemon.port are restart-required fields — warned but not applied on reload; identity and trust store never reloaded on SIGHUP/ReloadConfig (D-05)
- PendingPeerInfo defined in ipc.rs adjacent to IpcResponse (not a separate module) — keeps related types together per D-03
- DaemonConfig.Default derive replaced with manual impl returning listen=true — bool derive default is false, which is wrong for P2P symmetric model (D-07)
- futures-util 0.3 added as workspace dep — provides SinkExt/StreamExt for FramedWrite/FramedRead in periphore-net (D-03 resolved)
- periphore-net Cargo.toml extended to 13 deps: added periphore-identity, periphore-trust, periphore-config, periphore-core, postcard, futures-util, tempfile (dev) — full Phase 6 dep list (D-20)

### Open TODOs

- WR-01 (Warning): IdentityStore::keypair is pub — exposes raw private key material; should be private with sign()/verifying_key() accessors
- WR-02 (Warning): build_border() panics on label > 13 chars — add assert!() to document invariant
- WR-03 (Warning): key file written without sync_all() — identity lost on power failure during first-run
- IN-03 (Info): missing key file 0600 permission test in identity test suite

### Blockers

(None)

---

## Session Continuity

### Last Session

- **Date:** 2026-04-27
- **Work done:** Phase 6 plan 01 executed — PendingPeerInfo + IpcResponse::PendingPeers (b54730a), DaemonConfig.listen=true default + periphore-net deps (21d7b87); 5 protocol tests + 14 config tests passing; cargo build --workspace green
- **Stopped at:** Completed 06-01-PLAN.md (protocol/config/net-crate foundations)
- **Next action:** Execute 06-02-PLAN.md (periphore-net codec and error types)
