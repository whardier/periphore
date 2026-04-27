---
phase: 06-tcp-peering
verified: 2026-04-27T10:00:00Z
status: human_needed
score: 9/9
overrides_applied: 0
human_verification:
  - test: "Two real machines connect and report connected status"
    expected: "Both machines running periphored with a [[peer]] entry pointing at each other successfully complete the identity handshake and log 'peer connected and trusted'"
    why_human: "SC1 requires real inter-machine TCP — cannot be verified in-process or with grep; no CI environment with two network-accessible machines"
  - test: "SSH tunnel forwarding works end-to-end"
    expected: "ssh -L 7888:localhost:7888 remote-host, then point local daemon at localhost:7888 peer — connection completes with no protocol changes"
    why_human: "SC2 requires an actual SSH tunnel and two machines; the TCP-only constraint is architecturally enforced (no UDP in codebase) but the tunnel path cannot be proven without real infrastructure"
  - test: "Linux SSH remote launch and supervision"
    expected: "SSH into a Linux machine, run periphored, disconnect SSH session, verify periphored keeps running (either via nohup or systemctl --user). The contrib/periphored.service unit file enables this."
    why_human: "SC3 requires an actual Linux machine with systemd or nohup access; cannot be verified from macOS build environment"
---

# Phase 6: TCP Peering Verification Report

**Phase Goal:** Two machines can establish a peer-to-peer TCP connection with identity verification, with the transport working through SSH tunnels.
**Verified:** 2026-04-27T10:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | IpcResponse::PendingPeers and PendingPeerInfo are defined and serialize correctly | VERIFIED | `crates/periphore-protocol/src/ipc.rs` lines 46, 65-66: `pub struct PendingPeerInfo` and `PendingPeers { peers: Vec<PendingPeerInfo> }` — roundtrip test passes |
| 2 | DaemonConfig.listen defaults to true | VERIFIED | `crates/periphore-config/src/schema.rs` lines 33-42: `#[serde(default = "default_listen")]` + manual `Default` impl returns `listen: true` — 3 config tests pass |
| 3 | periphore-net crate has all required dependencies | VERIFIED | `crates/periphore-net/Cargo.toml` lines 15-26: all 5 internal crates + postcard + futures-util declared |
| 4 | NetError enum has all 8 variants; codec provides split_framed, encode/decode with 64KB cap | VERIFIED | `error.rs` lines 7-38: all 8 variants; `codec.rs` lines 19-57: MAX_FRAME_LENGTH=64*1024, split_framed, encode_message, decode_message — 9 codec tests pass |
| 5 | PeerEvent, HandshakeResult, PendingPeer, ActiveConn, ConnectionControl types defined | VERIFIED | `event.rs` line 12: PeerEvent with 3 variants; `connection.rs` lines 14-57: all types — 11 type tests pass |
| 6 | Handshake protocol: Hello/HelloAck with version check, fingerprint conflict, trust lookup, timeout | VERIFIED | `handshake.rs` lines 28-252: PROTOCOL_VERSION=1, both initiator and responder with 10s timeout, ProtocolVersion/FingerprintConflict error paths, identicon_from_fingerprint for pending peers — integration tests: handshake_trusted_peer, handshake_unknown_peer_goes_pending, protocol_version_mismatch, fingerprint_conflict all pass |
| 7 | ConnectionManager: TCP_NODELAY first, exponential backoff, CancellationToken, promote_pending, pending_list, cancel_peer | VERIFIED | `manager.rs` lines 99, 271: set_nodelay(true) in both accept and connect paths; lines 34-36: BACKOFF_INITIAL_MS=1000, BACKOFF_CAP_MS=30000; lines 399-400: token.cancelled(); all 5 methods confirmed — promote_pending integration test passes |
| 8 | periphored wired: macOS SSH guard, ConnectionManager init, net_event select arm, AcceptFingerprint→promote_pending, GetPendingVerifications→pending_list, D-11 peer diff in both reload paths | VERIFIED | `main.rs` lines 43-52: macOS cfg-gated IsTerminal check; line 141: ConnectionManager::new; lines 223-245: net_event arm with PeerPending WARN + PeerDisconnected reclaim; lines 287-295: AcceptFingerprint + promote_pending; lines 312-313: GetPendingVerifications→PendingPeers; lines 204-216, 324-335: cancel_peer in BOTH SIGHUP and ReloadConfig arms |
| 9 | Integration tests green: 6 NET-01 handshake tests + 2 NET-03/D-03 wiring tests | VERIFIED | `cargo test --workspace` exits 0: 92 tests, 0 failures. periphore-net integration: 6/6; periphored net_wiring: 2/2 |

**Score:** 9/9 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/periphore-protocol/src/ipc.rs` | PendingPeerInfo struct + PendingPeers variant | VERIFIED | Lines 46-66: both present, derives Serialize/Deserialize |
| `crates/periphore-config/src/schema.rs` | daemon.listen bool with default=true | VERIFIED | Lines 33-47: serde default + manual Default impl |
| `crates/periphore-net/Cargo.toml` | Full dep list (13 deps) | VERIFIED | Lines 15-26: all 5 internal crates + postcard + futures-util + anyhow |
| `crates/periphore-net/src/error.rs` | NetError thiserror enum (8 variants) | VERIFIED | Lines 7-38: all 8 variants including ProtocolVersion{expected,got} |
| `crates/periphore-net/src/codec.rs` | split_framed, encode/decode, MAX_FRAME_LENGTH | VERIFIED | Lines 19-57: max_frame_length(64*1024), CALLER RESPONSIBILITY comment |
| `crates/periphore-net/src/event.rs` | PeerEvent enum | VERIFIED | Lines 12-35: PeerPending, PeerConnected, PeerDisconnected |
| `crates/periphore-net/src/connection.rs` | HandshakeResult, PendingPeer, ActiveConn, ConnectionControl | VERIFIED | Lines 14-57: all types with promote_tx: mpsc::Sender<ConnectionControl> |
| `crates/periphore-net/src/lib.rs` | Module declarations + pub exports + DEFAULT_PORT=7888 | VERIFIED | Lines 12-27: all modules declared, all types re-exported, DEFAULT_PORT=7888 |
| `crates/periphore-identity/src/lib.rs` | identicon_from_fingerprint + word_phrase_from_fingerprint | VERIFIED | Lines 145, 155: both public free functions present |
| `crates/periphore-net/src/handshake.rs` | perform_handshake_initiator, perform_handshake_responder, PROTOCOL_VERSION | VERIFIED | Lines 28, 49, 150: all three present with full implementation |
| `crates/periphore-net/src/manager.rs` | ConnectionManager with all 5 methods + TCP_NODELAY + backoff | VERIFIED | Lines 46-466: full implementation; set_nodelay(true) at lines 99 and 271 |
| `crates/periphored/Cargo.toml` | periphore-net + periphore-core workspace deps | VERIFIED | Lines 19, 22: both deps declared |
| `crates/periphored/src/main.rs` | Full Phase 6 daemon wiring | VERIFIED | All required patterns found and wired |
| `contrib/periphored.service` | systemd user unit | VERIFIED | Type=simple, WantedBy=default.target, Restart=on-failure, ExecStart=%h/.cargo/bin/periphored, NoNewPrivileges=true, loginctl enable-linger instructions |
| `crates/periphore-net/tests/integration.rs` | 6 NET-01 handshake tests | VERIFIED | All 6 tests: handshake_trusted_peer, handshake_unknown_peer_goes_pending, protocol_version_mismatch, codec_roundtrip_hello, fingerprint_conflict, promote_pending — 6/6 pass |
| `crates/periphored/tests/net_wiring.rs` | NET-03 + D-03 IPC tests | VERIFIED | Both tests: peer_config_with_host_triggers_connector, pending_verifications_ipc — 2/2 pass |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/periphore-protocol/src/ipc.rs` | `crates/periphore-net/src/manager.rs` | PendingPeerInfo type used in pending_list() | VERIFIED | manager.rs imports PendingPeerInfo from periphore_protocol; pending_list() returns Vec<PendingPeerInfo> |
| `crates/periphore-config/src/schema.rs` | `crates/periphored/src/main.rs` | config.daemon.listen gates spawn_listener | VERIFIED | main.rs line 145: `if config.daemon.listen { conn_mgr.spawn_listener(...) }` |
| `crates/periphore-net/src/codec.rs` | `crates/periphore-net/src/handshake.rs` | split_framed() called to get typed read/write halves | VERIFIED | handshake.rs parameters accept FramedRead/FramedWrite; manager.rs calls codec::split_framed before passing to handshake |
| `crates/periphore-net/src/connection.rs` | `crates/periphore-net/src/handshake.rs` | HandshakeResult::Pending carries PendingPeer data | VERIFIED | handshake.rs returns HandshakeResult::Pending with identicon/word_phrase from free functions |
| `crates/periphore-net/src/handshake.rs` | `crates/periphore-net/src/manager.rs` | perform_handshake_* called from manager's spawned tasks | VERIFIED | manager.rs lines 117, 282: handshake::perform_handshake_responder and perform_handshake_initiator called |
| `crates/periphore-net/src/manager.rs` | `crates/periphored/src/main.rs` | ConnectionManager::new(event_tx) initialized; spawn_listener/spawn_connector called | VERIFIED | main.rs lines 141, 150, 162: ConnectionManager::new, spawn_listener, spawn_connector |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `main.rs` GetPendingVerifications arm | `peers` from `conn_mgr.pending_list()` | `ConnectionManager::pending: Arc<Mutex<HashMap<String, PendingPeer>>>` populated by spawned tasks | Yes — populated by handshake tasks inserting PendingPeer entries | FLOWING |
| `main.rs` net_event arm | PeerEvent from `net_event_rx.recv()` | `mpsc::Sender<PeerEvent>` in ConnectionManager, sent from spawn_listener/spawn_connector tasks after handshake | Yes — real TCP connection events | FLOWING |
| `integration.rs` promote_pending test | `connected_event` from `event_rx.recv()` | In-process TCP connection via ConnectionManager | Yes — real TCP handshake in-process | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| periphore-net integration tests pass | `cargo test -p periphore-net --test integration` | 6 passed, 0 failed | PASS |
| periphored net_wiring tests pass | `cargo test -p periphored --test net_wiring` | 2 passed, 0 failed | PASS |
| Full workspace tests pass | `cargo test --workspace` | 92 passed, 0 failed | PASS |
| periphored binary builds | `cargo build -p periphored` | Finished dev profile successfully | PASS |
| macOS SSH guard fires correctly | `target/debug/periphored --help` (non-TTY context) | Prints clear error and exits 1 | PASS |

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| NET-01 | 06-01, 06-02, 06-03, 06-05 | Two machines establish a peer connection over TCP | SATISFIED | handshake.rs implements Hello/HelloAck; integration tests prove in-process handshake; real two-machine test needs human (SC1) |
| NET-03 | 06-01, 06-02, 06-03, 06-04, 06-05 | Manual host definition works as alternative to auto-discovery | SATISFIED | PeerConfig.host triggers spawn_connector; peer_config_with_host_triggers_connector test passes |
| NET-04 | 06-01, 06-02 | Connections are SSH-tunnelable (TCP-only transport, no UDP) | SATISFIED | No UdpSocket anywhere in periphore-net; pure TCP + LengthDelimitedCodec; tunnel proof needs human (SC2) |
| NET-05 | 06-04 | On Linux with X-Auth, service can be launched remotely via SSH | SATISFIED | contrib/periphored.service systemd user unit with loginctl enable-linger instructions; live test needs human (SC3) |
| NET-06 | 06-04 | On other systems (macOS), daemon must be pre-running; remote launch produces clear error | SATISFIED | macOS cfg-gated IsTerminal check at top of main() before any async setup; exits with code 1 and clear message |

### Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| `crates/periphore-net/src/manager.rs` line 54 | `active: HashMap<String, ActiveConn>` — dead_code warning | Info | Intentional Phase 9 placeholder (input forwarding channel added in Phase 9); documented in SUMMARY and plan |
| `crates/periphore-net/src/manager.rs` lines 147, 312 | `// Ignore non-handshake frames in Phase 6` comments | Info | Intentional Phase 9 stub — active connections hold TCP open but don't process frames until Phase 9 wires input forwarding |
| `crates/periphored/src/main.rs` line 269 | `focus_sm.transfer_to()` never called — only reclaim() wired | Info | Intentional Phase 8 stub — topology routing (which would call transfer_to) is Phase 8; FocusStateMachine initialization and reclaim are correctly wired for Phase 6 |

None of these are blockers. All are documented deferred items with clear forward-phase ownership.

### Human Verification Required

#### 1. Two-machine real TCP connection (SC1)

**Test:** On two machines on the same network (or via tunnel), start `periphored` on each with a `[[peer]]` entry pointing to the other's IP and port 7888. Run `periphore status` or watch daemon logs on both sides.
**Expected:** Both daemons log "peer connected and trusted" (for pre-trusted peers) or "unknown peer pending verification — run: periphore trust accept..." (for first connection). Peer shows up in `periphore peers` output.
**Why human:** Requires two physical or virtual machines with network connectivity. In-process integration tests prove the handshake protocol works; the two-machine case adds OS TCP stack, firewall rules, and routing — these cannot be simulated by grep.

#### 2. SSH tunnel forwarding (SC2)

**Test:** On a remote Linux machine, ensure periphored is running on port 7888. From a local machine: `ssh -L 7889:localhost:7888 remote-user@remote-host`. Configure local periphored with `[[peer]]` pointing to `localhost:7889`. Start local periphored and watch for handshake completion.
**Expected:** Connection completes through the tunnel with the same handshake flow as a direct connection. No protocol changes required.
**Why human:** Requires an SSH-accessible remote machine, firewall configuration, and actual tunnel setup. The TCP-only design constraint (no UDP) is architecturally enforced in code, but end-to-end tunnel operation cannot be proven without real infrastructure.

#### 3. Linux SSH remote supervision (SC3)

**Test:** On a Linux machine: (a) Install `contrib/periphored.service` to `~/.config/systemd/user/periphored.service`, run `systemctl --user enable --now periphored`, disconnect SSH, reconnect and verify `systemctl --user status periphored` shows active. (b) Alternative: `nohup periphored > periphored.log 2>&1 &`, disconnect SSH, reconnect, verify process still running.
**Expected:** periphored continues running after SSH session ends; logs accessible via journalctl or nohup output file.
**Why human:** Requires a Linux machine with systemd user session or nohup capability; SSH session lifecycle cannot be simulated programmatically in this environment.

### Gaps Summary

No gaps found. All 9 must-haves are verified against the actual codebase. The 3 human verification items above are genuine runtime behaviors that cannot be confirmed without real two-machine infrastructure — they are not code gaps.

The phase goal "Two machines can establish a peer-to-peer TCP connection with identity verification, with the transport working through SSH tunnels" is architecturally complete and proven correct by in-process integration tests. The human verification items confirm the real-world operational deployment, not the protocol correctness.

---

_Verified: 2026-04-27T10:00:00Z_
_Verifier: Claude (gsd-verifier)_
