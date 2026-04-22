# Requirements — Periphore

**Project:** Periphore — Peer-to-peer input sharing daemon
**Milestone:** v1 — Core peering, topology, and captive-window input
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
- [ ] **SEC-03**: Fingerprint available as typed word phrase (one side reads, other types — not displayed simultaneously)
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
- [ ] **INP-04**: Captive window mode — fullscreen/kiosk captures input without requiring accessibility permissions
- [ ] **INP-05**: Hotkey escapes captive mode and returns input to local machine

### Configuration (CFG)

- [ ] **CFG-01**: System never auto-writes configuration; all config is user-authored
- [ ] **CFG-02**: Hard config conflicts between peers prevent peering
- [ ] **CFG-03**: Config can define preferred monitor layouts for dynamic monitor scenarios

---

## v2 Requirements (Deferred)

- Seamless edge crossing via OS accessibility APIs (CGEvent tap / evdev) — deferred after captive window is stable
- Text clipboard sharing — deferred past v1 service layer
- Reconnection with full state recovery — reliability polish after core is stable
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

*Filled by roadmapper — maps each REQ-ID to the phase that delivers it.*

| REQ-ID | Phase | Notes |
|--------|-------|-------|
| NET-01 | — | |
| NET-02 | — | |
| NET-03 | — | |
| NET-04 | — | |
| NET-05 | — | |
| NET-06 | — | |
| IPC-01 | — | |
| IPC-02 | — | |
| SEC-01 | — | |
| SEC-02 | — | |
| SEC-03 | — | |
| SEC-04 | — | |
| SEC-05 | — | |
| SEC-06 | — | |
| TOP-01 | — | |
| TOP-02 | — | |
| TOP-03 | — | |
| TOP-04 | — | |
| TOP-05 | — | |
| TOP-06 | — | |
| TOP-07 | — | |
| TOP-08 | — | |
| INP-01 | — | |
| INP-02 | — | |
| INP-03 | — | |
| INP-04 | — | |
| INP-05 | — | |
| CFG-01 | — | |
| CFG-02 | — | |
| CFG-03 | — | |
