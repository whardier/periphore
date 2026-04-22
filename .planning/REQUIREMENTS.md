# Requirements -- Periphore

**Project:** Periphore -- Peer-to-peer input sharing daemon
**Milestone:** v1 -- Core peering, topology, and captive-window input
**Generated:** 2026-04-22

---

## v1 Requirements

### Transport (NET)

- [ ] **NET-01**: Two machines establish a peer connection over TCP
- [ ] **NET-02**: Auto-discovery locates peers on the local network via mDNS
- [ ] **NET-03**: Manual host definition works as alternative to auto-discovery
- [ ] **NET-04**: Connections are SSH-tunnelable (TCP-only transport, no UDP)
- [ ] **NET-05**: On Linux with X-Auth, service can be launched and supervised remotely via SSH
- [ ] **NET-06**: On other systems, daemon must be pre-running; listens on IPC + TCP

### IPC Layer (IPC)

- [ ] **IPC-01**: Service exposes a Unix domain socket (platform-appropriate) for local IPC
- [ ] **IPC-02**: IPC layer is the modular boundary between transport and capture, testable without a network peer

### Security & Identity (SEC)

- [ ] **SEC-01**: Each node generates a persistent Ed25519 keypair; fingerprint derived from public key
- [ ] **SEC-02**: Fingerprint displayed as identicon (visual, shown on both machines simultaneously)
- [ ] **SEC-03**: Fingerprint available as typed word phrase (one side reads, other types -- not displayed simultaneously)
- [ ] **SEC-04**: Identicon display can be disabled for headless/automated setups
- [ ] **SEC-05**: Accepted fingerprints cached between sessions (no auto-write to main config)
- [ ] **SEC-06**: Hard configuration can include peer fingerprint; conflicts prevent peering

### Monitor Topology (TOP)

- [ ] **TOP-01**: Service queries all monitors on the local machine at startup
- [ ] **TOP-02**: Peer topology (monitor layout relative to peer monitors) configured via config file
- [ ] **TOP-03**: Peers exchange and negotiate topology on connection
- [ ] **TOP-04**: CLI debug output shows resolved topology when debug logging is enabled
- [ ] **TOP-05**: Multi-monitor setups supported (N monitors on one machine, M on another)
- [ ] **TOP-06**: Corner resolution and offset compensation for mismatched monitor arrangements
- [ ] **TOP-07**: Edge definitions are directional per monitor (left/right/top/bottom of a specific monitor)
- [ ] **TOP-08**: Smart cycling: system understands when edge traversal has looped back around

### Input Flow (INP)

- [ ] **INP-01**: Source machine captures input; sink machine injects it
- [ ] **INP-02**: Bi-directional control (either machine as source or sink) is optional per session
- [ ] **INP-03**: Input treated as relative coordinates (not absolute) for cross-machine movement
- [ ] **INP-04**: Captive window mode -- fullscreen/kiosk captures input without requiring accessibility permissions
- [ ] **INP-05**: Hotkey escapes captive mode and returns input to local machine

### Configuration (CFG)

- [ ] **CFG-01**: System never auto-writes configuration; all config is user-authored
- [ ] **CFG-02**: Hard config conflicts between peers prevent peering
- [ ] **CFG-03**: Config can define preferred monitor layouts for dynamic monitor scenarios

---

## v2 Requirements (Deferred)

- Seamless edge crossing via OS accessibility APIs (CGEvent tap / evdev) -- deferred after captive window is stable
- Text clipboard sharing -- deferred past v1 service layer
- Reconnection with full state recovery -- reliability polish after core is stable
- Dynamic monitor add/remove detection and automatic re-layout
- Wayland-specific input handling

---

## Out of Scope

| Exclusion | Reason |
|-----------|--------|
| Windows support | macOS + Linux only for v1; triples platform matrix |
| GUI topology visualizer | Deferred to dedicated GUI phase; v1 is headless daemon + CLI only |
| Web/browser-based client | Separate project; browser input capture and WebRTC latency are separate concerns |
| Audio forwarding | Different latency requirements, codec needs, and platform APIs |
| USB peripheral pass-through | Extension point only; no v1 implementation |
| Drag-and-drop file transfer | Enormously complex and OS-specific; out of scope entirely |
| Gamepad/joystick forwarding | Different input class, different latency requirements |
| Automatic physical-position detection | Unsolvable without user input |

---

## Traceability

*Mapped by roadmapper on 2026-04-22.*

| REQ-ID | Phase | Status |
|--------|-------|--------|
| NET-01 | Phase 6: TCP Peering | Pending |
| NET-02 | Phase 7: Peer Discovery | Pending |
| NET-03 | Phase 6: TCP Peering | Pending |
| NET-04 | Phase 6: TCP Peering | Pending |
| NET-05 | Phase 6: TCP Peering | Pending |
| NET-06 | Phase 6: TCP Peering | Pending |
| IPC-01 | Phase 4: IPC Layer | Pending |
| IPC-02 | Phase 4: IPC Layer | Pending |
| SEC-01 | Phase 2: Identity & Cryptography | Pending |
| SEC-02 | Phase 2: Identity & Cryptography | Pending |
| SEC-03 | Phase 2: Identity & Cryptography | Pending |
| SEC-04 | Phase 2: Identity & Cryptography | Pending |
| SEC-05 | Phase 3: Configuration & Trust Persistence | Pending |
| SEC-06 | Phase 3: Configuration & Trust Persistence | Pending |
| TOP-01 | Phase 8: Monitor Topology | Pending |
| TOP-02 | Phase 8: Monitor Topology | Pending |
| TOP-03 | Phase 8: Monitor Topology | Pending |
| TOP-04 | Phase 5: CLI Tool (periphore-ctl) | Pending |
| TOP-05 | Phase 8: Monitor Topology | Pending |
| TOP-06 | Phase 8: Monitor Topology | Pending |
| TOP-07 | Phase 8: Monitor Topology | Pending |
| TOP-08 | Phase 8: Monitor Topology | Pending |
| INP-01 | Phase 9: Input Capture & Injection | Pending |
| INP-02 | Phase 9: Input Capture & Injection | Pending |
| INP-03 | Phase 9: Input Capture & Injection | Pending |
| INP-04 | Phase 10: Captive Window Mode | Pending |
| INP-05 | Phase 10: Captive Window Mode | Pending |
| CFG-01 | Phase 1: Workspace & Protocol Foundation | Pending |
| CFG-02 | Phase 3: Configuration & Trust Persistence | Pending |
| CFG-03 | Phase 3: Configuration & Trust Persistence | Pending |
