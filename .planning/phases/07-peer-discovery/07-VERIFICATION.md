---
phase: 07-peer-discovery
verified: 2026-04-28T19:30:00Z
status: human_needed
score: 10/12 must-haves verified
overrides_applied: 0
human_verification:
  - test: "On a local subnet with two machines, start both daemons with [discovery] enabled = true, then run periphore peers discovered on one machine"
    expected: "The other daemon appears in the discovered list within 5 seconds (ROADMAP SC1)"
    why_human: "Requires two networked machines; cannot verify multicast mDNS propagation in a single-machine test environment"
  - test: "On a corporate or firewalled network where mDNS is blocked, start the daemon with discovery enabled, then run periphore peers discovered"
    expected: "Daemon starts normally, logs a tracing::warn! about mDNS failure, and manual [[peer]] host= config still connects successfully (ROADMAP SC3)"
    why_human: "Requires a restricted network environment; mDNS failure path uses real network binding that cannot be simulated in unit tests"
---

# Phase 7: Peer Discovery Verification Report

**Phase Goal:** Passive peer discovery — mDNS + SSH probe, CLI visibility, no auto-connect
**Verified:** 2026-04-28T19:30:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | D-01: periphore-discovery crate scaffold created at crates/periphore-discovery | VERIFIED | Directory exists with all 5 source modules: lib.rs, error.rs, list.rs, mdns.rs, probe.rs |
| 2 | D-02: periphore-discovery added to workspace Cargo.toml dependencies; mdns-sd workspace dep added | VERIFIED | Cargo.toml:24 `periphore-discovery = { path = ... version = "0.1.0" }`; Cargo.toml:49 `mdns-sd = { version = "0.19" }` |
| 3 | D-03: DiscoveryConfig.enabled defaults false (opt-in, CFG-01 compliant) | VERIFIED | schema.rs:176 `enabled: false` in Default impl; struct derives only Deserialize, not Serialize (CFG-01 enforced at compile time) |
| 4 | D-04: DiscoveryConfig struct exists with all 5 fields (enabled, instance_name, service_type, ssh_probe_enabled, ssh_probe_ports) | VERIFIED | schema.rs:139-163 — all 5 fields present with correct types and serde defaults |
| 5 | D-05: DiscoveryService is passive — maintains in-memory list only, no auto-connect | VERIFIED | main.rs:281-291 PeerDiscovered event logs only (tracing::info!); no call to ConnectionManager or TCP connect; lib.rs doc comment confirms passive model |
| 6 | D-06: IpcRequest::GetDiscoveredPeers and IpcResponse::DiscoveredPeers variants exist; IpcCommand::GetDiscoveredPeers in periphore-ipc with oneshot responder | VERIFIED | ipc.rs:32 (request), ipc.rs:54-87 (struct + response); lib.rs:63-65 (command variant); lib.rs:113-115 (match arm) |
| 7 | D-07/D-08: Hybrid expiry — mDNS goodbye fires immediate removal; TTL GC sweeps at 30s interval; TTL is 5 minutes (300s) | VERIFIED | list.rs:13 MAX_PEERS=64; list.rs:16 TTL=Duration::from_secs(300); lib.rs:122-148 GC task spawned always at 30s interval; list.rs:126-137 remove_by_fullname() for goodbye events |
| 8 | D-09: Discovered peer list caps at 64 entries; oldest evicted on overflow with tracing::warn! | VERIFIED | list.rs:90-101 cap enforcement with warn! on eviction; integration test list_cap_eviction passes |
| 9 | D-10: periphore peers discovered subcommand sends GetDiscoveredPeers and displays table; shows hint when empty | VERIFIED | discovered.rs:20 ipc_request call; discovered.rs:22-45 table output; discovered.rs:24-33 empty-list hint with config snippet |
| 10 | D-10/D-11: periphore peers pending sends GetPendingVerifications and displays fingerprint + word phrase | VERIFIED | pending.rs:20 ipc_request call; pending.rs:22-43 fingerprint + word phrase + identicon display; pending.rs:42 trust accept hint |
| 11 | ROADMAP SC1: Daemon broadcasts presence via mDNS and appears in another daemon's peer list within 5 seconds | UNCERTAIN | mdns_register_and_browse() registers service via ServiceDaemon + ServiceInfo; browse loop handles ServiceResolved; correctness on real subnet requires human verification |
| 12 | ROADMAP SC3: mDNS failure logs warn and daemon continues; manual host config works as fallback | UNCERTAIN | Code path verified (mdns.rs:37, mdns.rs:98 log warn and return Ok); real network failure behavior requires human verification on a restricted network |

**Score:** 10/12 truths verified (2 require human testing on real network)

---

### Note on ROADMAP SC2

ROADMAP SC2 states: "Discovered peers proceed through the same identity verification handshake as manually-configured peers."

The Phase 7 design (D-05, explicit in 07-CONTEXT.md) chose a passive model: discovered peers are listed in the CLI but NOT auto-connected. Users add discovered peers to config manually, then Phase 6 TCP Peering handles the identity handshake on connection. The CONTEXT document explicitly marks auto-connect as "out of scope for Phase 7."

This means SC2 is satisfied architecturally (the same handshake path is used), but not automatically on discovery. This is an intentional design scope decision, not an implementation gap. SC2 is counted as verified because the handshake infrastructure (Phase 6 TCP Peering, plans 06-01 and 06-02 complete) exists and discovered peers do go through it when connected — the user must initiate the connection. No override is needed as the phase goal explicitly states "no auto-connect."

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/periphore-config/src/schema.rs` | DiscoveryConfig struct | VERIFIED | pub struct DiscoveryConfig at line 139, all 5 fields, default enabled=false |
| `crates/periphore-protocol/src/ipc.rs` | GetDiscoveredPeers IPC variant + DiscoveredPeerInfo struct | VERIFIED | GetDiscoveredPeers in IpcRequest (line 32), DiscoveredPeerInfo struct (lines 54-62), DiscoveredPeers in IpcResponse (lines 83-87) |
| `crates/periphore-ipc/src/lib.rs` | IpcCommand::GetDiscoveredPeers with responder | VERIFIED | Variant at line 63 with oneshot::Sender<IpcResponse>; match arm at lines 113-115 |
| `Cargo.toml` | Workspace deps for mdns-sd and periphore-discovery | VERIFIED | Both present at lines 24 and 49 |
| `crates/periphore-discovery/Cargo.toml` | Crate manifest with all required deps | VERIFIED | Contains name, mdns-sd, periphore-net, periphore-config, periphore-protocol, periphore-identity |
| `crates/periphore-discovery/src/lib.rs` | DiscoveryService struct, DiscoveryEvent enum | VERIFIED | DiscoveryService at line 51, DiscoveryEvent at line 29; start() and discovered_list() methods |
| `crates/periphore-discovery/src/error.rs` | DiscoveryError thiserror enum | VERIFIED | pub enum DiscoveryError with MdnsInit, MdnsBrowse, MdnsRegister, Io, Internal variants |
| `crates/periphore-discovery/src/list.rs` | DiscoveredPeerList with cap/GC/snapshot | VERIFIED | MAX_PEERS=64, TTL=300s, upsert/remove/remove_by_fullname/gc/snapshot methods present |
| `crates/periphore-discovery/src/mdns.rs` | mDNS registration + browse loop | VERIFIED | mdns_register_and_browse() at line 25; ServiceResolved/ServiceRemoved handled |
| `crates/periphore-discovery/src/probe.rs` | SSH tunnel port probe loop | VERIFIED | ssh_probe_loop() at line 40; probe_handshake() with Hello/HelloAck; own_fingerprint check at line 73 |
| `crates/periphore-cli/src/cli.rs` | Peers subcommand group with PeersAction enum | VERIFIED | Peers { action: PeersAction } at line 42; PeersAction at line 63 |
| `crates/periphore-cli/src/commands/peers/discovered.rs` | Handler for periphore peers discovered | VERIFIED | ipc_request call with GetDiscoveredPeers; table output; empty-list hint |
| `crates/periphore-cli/src/commands/peers/pending.rs` | Handler for periphore peers pending | VERIFIED | ipc_request with GetPendingVerifications; fingerprint + word phrase display |
| `crates/periphore-cli/src/commands/peers/mod.rs` | Module declarations | VERIFIED | pub(crate) mod discovered; pub(crate) mod pending |
| `crates/periphore-cli/src/lib.rs` | Dispatch for Peers command | VERIFIED | Commands::Peers arm at line 31 routing to discovered/pending handlers |
| `crates/periphored/Cargo.toml` | periphore-discovery dependency added | VERIFIED | periphore-discovery at line 20 |
| `crates/periphored/src/main.rs` | Discovery service wiring + IPC dispatch + event handling | VERIFIED | DiscoveryService::new() at line 177; start() at line 183; discovery_event select! arm at line 279; GetDiscoveredPeers dispatch at line 374; discovery_cancel.cancel() at line 451 |
| `crates/periphore-discovery/tests/integration.rs` | 7 integration tests | VERIFIED | All 7 tests present and passing (cargo test -p periphore-discovery: 7/7 ok) |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/periphore-ipc/src/lib.rs` | `crates/periphore-protocol/src/ipc.rs` | `IpcRequest::GetDiscoveredPeers =>` match arm | WIRED | lib.rs:113-115 match arm confirmed |
| `crates/periphore-discovery/src/lib.rs` | `crates/periphore-discovery/src/mdns.rs` | `DiscoveryService::start()` spawns mDNS task | WIRED | lib.rs:90-97 `tasks.spawn(mdns::mdns_register_and_browse(...))` |
| `crates/periphore-discovery/src/lib.rs` | `crates/periphore-discovery/src/probe.rs` | `DiscoveryService::start()` spawns SSH probe task | WIRED | lib.rs:109-116 `tasks.spawn(probe::ssh_probe_loop(...))` |
| `crates/periphore-discovery/src/lib.rs` | `crates/periphore-discovery/src/list.rs` | `DiscoveryService` holds `Arc<Mutex<DiscoveredPeerList>>` | WIRED | lib.rs:52 `peers: Arc<std::sync::Mutex<DiscoveredPeerList>>` |
| `crates/periphore-cli/src/commands/peers/discovered.rs` | `crates/periphore-cli/src/client.rs` | `ipc_request(socket_path, IpcRequest::GetDiscoveredPeers)` | WIRED | discovered.rs:20 confirmed |
| `crates/periphore-cli/src/commands/peers/pending.rs` | `crates/periphore-cli/src/client.rs` | `ipc_request(socket_path, IpcRequest::GetPendingVerifications)` | WIRED | pending.rs:20 confirmed |
| `crates/periphored/src/main.rs` | `crates/periphore-discovery/src/lib.rs` | `periphore_discovery::DiscoveryService::new() + .start() + .discovered_list()` | WIRED | main.rs:177, 183, 376 |
| `crates/periphored/src/main.rs` | `crates/periphore-ipc/src/lib.rs` | `IpcCommand::GetDiscoveredPeers` dispatch arm | WIRED | main.rs:374-377 |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|-------------------|--------|
| `discovered.rs` | `peers: Vec<DiscoveredPeerInfo>` | IPC call to daemon → `discovery_service.discovered_list()` → `DiscoveredPeerList::snapshot()` | Yes — in-memory list populated by mDNS/SSH probe tasks | FLOWING |
| `main.rs::DiscoveryService.discovered_list()` | In-memory `DiscoveredPeerList` | `mdns_register_and_browse()` calls `list.upsert()` on ServiceResolved; `ssh_probe_loop()` calls `list.upsert()` on HelloAck | Yes — real mDNS events and TCP handshake populate list | FLOWING |
| `pending.rs` | `peers: Vec<PendingPeerInfo>` | IPC call to daemon → `conn_mgr.pending_list()` | Yes — ConnectionManager populates on incoming connections (Phase 6) | FLOWING |

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full workspace builds | `cargo build --workspace` | `Finished dev profile [unoptimized + debuginfo]` | PASS |
| periphore-discovery tests | `cargo test -p periphore-discovery` | 7/7 tests pass | PASS |
| Full workspace tests | `cargo test --workspace` | All test suites green, 0 failures | PASS |
| list_cap_eviction (D-09) | integration test | 64-peer cap enforced, overflow evicts oldest | PASS |
| gc_removes_expired (D-08) | integration test | GC does not remove fresh entries; TTL=300s constant confirmed | PASS |
| remove_by_fullname (D-07) | integration test | mDNS goodbye removal via fullname match works | PASS |
| ssh_probe_against_test_listener | integration test | Real Hello/HelloAck handshake probe discovers test listener | PASS |
| ssh_probe_skips_own_fingerprint (Pitfall 3) | integration test | Own fingerprint not added to discovered list | PASS |
| mDNS real-network broadcast | Requires two machines on same subnet | Not testable in single-machine environment | SKIP (human) |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| NET-02 | 07-01, 07-02, 07-03, 07-04 | Auto-discovery locates peers on the local network via mDNS | SATISFIED | periphore-discovery crate with mDNS register/browse; SSH probe; CLI visibility; daemon wired; 7 integration tests passing |

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/periphore-discovery/src/probe.rs` | 148-149 | `// NOTE: identity.keypair is pub (WR-01 open TODO)` | Info | Pre-existing WR-01 warning from Phase 6; not a Phase 7 stub; keypair access works correctly now |

No stub, placeholder, or orphaned implementation patterns found in Phase 7 artifacts. All handlers produce real output (not hardcoded empty returns). The `DiscoveredPeerList` is populated by live mDNS/SSH probe events, not static data.

---

### Human Verification Required

#### 1. mDNS Real-Network Broadcast (ROADMAP SC1)

**Test:** On a local subnet with two machines, start both daemons with `[discovery]\nenabled = true` in config, then run `periphore peers discovered` on either machine.
**Expected:** The other daemon's hostname and port appear in the discovered list within 5 seconds.
**Why human:** mDNS uses IP multicast which requires real network interfaces and another machine. The integration tests use SSH probe (localhost loopback), not mDNS multicast. The mDNS code path (mdns_register_and_browse) is correct but its real-world behavior on an actual subnet cannot be verified without physical machines.

#### 2. mDNS Silent Failure Fallback (ROADMAP SC3)

**Test:** On a corporate or firewalled network where multicast DNS is blocked, start the daemon with `[discovery]\nenabled = true` and a configured `[[peer]]` host entry. Observe daemon startup logs and attempt to connect to the manual peer.
**Expected:** Daemon starts normally with `WARN mDNS daemon failed to start — discovery unavailable on this network`, and the manual peer connection proceeds normally.
**Why human:** The code path that logs warn and continues (mdns.rs:37) is correct and verified by code inspection, but actual behavior on a restricted network requires a real network environment with multicast blocked.

---

### Gaps Summary

No structural gaps found. All planned artifacts exist, are substantive (not stubs), and are correctly wired. The `cargo build --workspace` and `cargo test --workspace` both pass. Two items require human verification on real network infrastructure — these are behavioral tests that cannot be automated without physical networked machines.

The ROADMAP SC2 ("discovered peers proceed through the same identity verification handshake") is satisfied architecturally: Phase 7 explicitly scoped as passive discovery (D-05); discovered peers go through the Phase 6 TCP handshake when a user adds them to config and restarts. This was an intentional design choice documented in 07-CONTEXT.md and 07-DISCUSSION-LOG.md.

---

_Verified: 2026-04-28T19:30:00Z_
_Verifier: Claude (gsd-verifier)_
