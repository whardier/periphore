# Roadmap -- Periphore

**Project:** Periphore -- Peer-to-peer input sharing daemon
**Milestone:** 1 -- v1 Core
**Granularity:** Fine
**Total phases:** 10
**Total requirements:** 30/30 mapped

---

## Phases

- [ ] **Phase 1: Workspace & Protocol Foundation** - Cargo workspace scaffold, protocol message types, config discipline
- [ ] **Phase 2: Identity & Cryptography** - Ed25519 keypairs, fingerprint derivation, identicon and word-phrase verification
- [ ] **Phase 3: Configuration & Trust Persistence** - Layered config loading, fingerprint caching, hard config conflict enforcement
- [ ] **Phase 4: IPC Layer** - Unix domain socket service boundary, modular transport/capture decoupling
- [ ] **Phase 5: CLI Tool (periphore-ctl)** - CLI binary for daemon interaction, debug topology output
- [ ] **Phase 6: TCP Peering** - TCP peer connections, SSH tunnelability, manual host definition, daemon lifecycle
- [ ] **Phase 7: Peer Discovery** - mDNS auto-discovery of peers on the local network
- [ ] **Phase 8: Monitor Topology** - Monitor enumeration, topology config/negotiation, multi-monitor edge resolution
- [ ] **Phase 9: Input Capture & Injection** - Source/sink input flow, relative coordinates, bidirectional control
- [ ] **Phase 10: Captive Window Mode** - Fullscreen/kiosk input capture, hotkey escape

---

## Phase Details

### Phase 1: Workspace & Protocol Foundation

**Goal:** The project has a buildable Cargo workspace with shared protocol types and config-discipline enforcement, so all subsequent crates have a foundation to build on.
**Depends on:** Nothing (first phase)
**Requirements:** CFG-01
**Success criteria:**
1. `cargo build --workspace` succeeds with at least `periphore-protocol` and `periphore-config` crates present
2. Protocol crate defines message envelope types (handshake, topology, input event, control) that compile and serialize/deserialize round-trip via `postcard`
3. Config crate loads a TOML config file with layered precedence (defaults < file < env < CLI) and never writes to disk under any code path
4. Running the workspace binary with `--help` produces usage output (proving the binary target exists)
**Plans:** TBD

### Phase 2: Identity & Cryptography

**Goal:** Every node can generate a persistent cryptographic identity and present its fingerprint visually (identicon) and verbally (word phrase) for peer verification.
**Depends on:** Phase 1
**Requirements:** SEC-01, SEC-02, SEC-03, SEC-04
**Success criteria:**
1. Running the daemon for the first time generates an Ed25519 keypair that persists across restarts in a well-known path
2. The fingerprint derived from the public key is deterministic -- same key always produces the same fingerprint on both macOS and Linux
3. The identicon rendered from a fingerprint is visually identical across platforms when compared side-by-side
4. The word phrase generated from a fingerprint produces the same words on both platforms and can be typed character-by-character for verification
5. Identicon display can be disabled via config or CLI flag, with word-phrase-only verification still functional
**Plans:** TBD

### Phase 3: Configuration & Trust Persistence

**Goal:** Users can manage peer trust through fingerprint caching and hard config enforcement, with config conflicts preventing unsafe peering.
**Depends on:** Phase 2
**Requirements:** SEC-05, SEC-06, CFG-02, CFG-03
**Success criteria:**
1. After accepting a peer's fingerprint, reconnecting to that same peer succeeds without re-verification (fingerprint is cached)
2. Cached fingerprints are stored in a separate file from the main config and survive daemon restarts
3. When a peer's fingerprint is hardcoded in config and the connecting peer presents a different fingerprint, the connection is refused with a clear error message
4. When two peers have conflicting hard config (e.g., both claim the same edge), peering is refused and the conflict is reported
5. Config can define preferred monitor layouts that are used when monitors change between sessions
**Plans:** TBD

### Phase 4: IPC Layer

**Goal:** The daemon exposes a local Unix domain socket that decouples transport from capture, enabling local tooling and testing without a network peer.
**Depends on:** Phase 1
**Requirements:** IPC-01, IPC-02
**Success criteria:**
1. The daemon creates a Unix domain socket at a platform-appropriate path on startup and removes it on clean shutdown
2. A local process can connect to the socket and exchange structured messages (request/response) with the daemon
3. The IPC layer can simulate peer input events locally without any TCP connection, proving the modular boundary works
**Plans:** TBD

### Phase 5: CLI Tool (periphore-ctl)

**Goal:** Users can interact with the running daemon through a CLI tool that communicates over IPC, including inspecting topology state.
**Depends on:** Phase 4
**Requirements:** TOP-04
**Success criteria:**
1. `periphore-ctl status` connects to the daemon via IPC and reports whether it is running and its identity fingerprint
2. `periphore-ctl topology` (or equivalent debug command) outputs the resolved monitor topology when debug logging is enabled
3. `periphore-ctl` fails gracefully with a clear error when the daemon is not running
**Plans:** TBD

### Phase 6: TCP Peering

**Goal:** Two machines can establish a peer-to-peer TCP connection with identity verification, with the transport working through SSH tunnels.
**Depends on:** Phase 2, Phase 4
**Requirements:** NET-01, NET-03, NET-04, NET-05, NET-06
**Success criteria:**
1. Two machines running the daemon can connect to each other by specifying the peer's IP/hostname, complete the identity handshake, and report "connected" status
2. The connection works through an SSH tunnel (`ssh -L` forwarding a local port to the remote daemon port) with no protocol changes
3. On Linux with X-Auth, the daemon can be launched remotely via SSH and supervised (stays running after SSH session ends)
4. On macOS, the daemon must be pre-running locally; attempting to launch it remotely produces a clear error explaining why
5. Manual host definition in config or CLI args successfully connects to a peer without requiring discovery
**Plans:** TBD

### Phase 7: Peer Discovery

**Goal:** Peers on the same local network find each other automatically via mDNS without manual configuration.
**Depends on:** Phase 6
**Requirements:** NET-02
**Success criteria:**
1. A daemon started with discovery enabled broadcasts its presence via mDNS and appears in another daemon's peer list within 5 seconds on the same subnet
2. Discovered peers proceed through the same identity verification handshake as manually-configured peers
3. When mDNS fails silently (corporate network, firewall), the daemon logs a warning and manual host config still works as fallback
**Plans:** TBD

### Phase 8: Monitor Topology

**Goal:** Peers exchange monitor layouts and negotiate a unified topology so cursor movement across screen edges maps correctly between machines.
**Depends on:** Phase 6
**Requirements:** TOP-01, TOP-02, TOP-03, TOP-05, TOP-06, TOP-07, TOP-08
**Success criteria:**
1. On startup, the daemon detects all connected monitors and their resolutions/positions on both macOS and Linux
2. Two peers with topology defined in their config files exchange layouts on connection and agree on a unified edge map
3. A machine with 2 monitors peered with a machine with 1 monitor correctly resolves which monitor edges connect to the peer
4. Edge definitions per monitor (left/right/top/bottom) map cursor exits to the correct peer monitor, including offset compensation for mismatched resolutions
5. When edge traversal would loop back to the originating machine, the system detects the cycle and handles it (no infinite loop)
**Plans:** TBD

### Phase 9: Input Capture & Injection

**Goal:** Input events flow from a source machine to a sink machine in real time, with relative coordinate translation and optional bidirectional control.
**Depends on:** Phase 8
**Requirements:** INP-01, INP-02, INP-03
**Success criteria:**
1. Keyboard events captured on the source machine are injected on the sink machine with correct key mapping (including modifiers)
2. Mouse movement captured on the source is injected on the sink using relative deltas, not absolute coordinates
3. When bidirectional mode is enabled, either machine can become the source or sink dynamically during a session
4. Input latency from capture to injection is under 10ms on a local network (TCP_NODELAY set)
**Plans:** TBD

### Phase 10: Captive Window Mode

**Goal:** Users can enter a fullscreen captive window that captures all input without requiring OS accessibility permissions, and escape it with a hotkey.
**Depends on:** Phase 9
**Requirements:** INP-04, INP-05
**Success criteria:**
1. Running the captive window command opens a fullscreen/kiosk window that grabs keyboard and mouse input
2. While in captive mode, all keyboard and mouse events are forwarded to the connected peer -- no input leaks to the local desktop
3. Pressing the configured hotkey exits captive mode and returns full input control to the local machine immediately
4. Captive mode works without any OS accessibility permissions on both macOS and Linux
**Plans:** TBD

---

## Coverage

| Category | Requirements | Phase(s) |
|----------|-------------|----------|
| Transport (NET) | NET-01, NET-03, NET-04, NET-05, NET-06 | Phase 6 |
| Transport (NET) | NET-02 | Phase 7 |
| IPC Layer (IPC) | IPC-01, IPC-02 | Phase 4 |
| Security (SEC) | SEC-01, SEC-02, SEC-03, SEC-04 | Phase 2 |
| Security (SEC) | SEC-05, SEC-06 | Phase 3 |
| Topology (TOP) | TOP-04 | Phase 5 |
| Topology (TOP) | TOP-01, TOP-02, TOP-03, TOP-05, TOP-06, TOP-07, TOP-08 | Phase 8 |
| Input Flow (INP) | INP-01, INP-02, INP-03 | Phase 9 |
| Input Flow (INP) | INP-04, INP-05 | Phase 10 |
| Configuration (CFG) | CFG-01 | Phase 1 |
| Configuration (CFG) | CFG-02, CFG-03 | Phase 3 |

**Mapped: 30/30 -- No orphaned requirements.**

---

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Workspace & Protocol Foundation | 0/? | Not started | - |
| 2. Identity & Cryptography | 0/? | Not started | - |
| 3. Configuration & Trust Persistence | 0/? | Not started | - |
| 4. IPC Layer | 0/? | Not started | - |
| 5. CLI Tool (periphore-ctl) | 0/? | Not started | - |
| 6. TCP Peering | 0/? | Not started | - |
| 7. Peer Discovery | 0/? | Not started | - |
| 8. Monitor Topology | 0/? | Not started | - |
| 9. Input Capture & Injection | 0/? | Not started | - |
| 10. Captive Window Mode | 0/? | Not started | - |
