# Feature Landscape — Periphore

**Domain:** Peer-to-peer input sharing (keyboard/mouse across machines)
**Researched:** 2026-04-22
**Competitive references:** Synergy (commercial), Barrier (OSS fork), Input Leap (Barrier successor), Deskflow (Synergy 3 OSS rebrand)

---

## Table Stakes

Features users expect. Missing any of these prevents adoption.

| Feature | Why Expected | Complexity | Existing Tool Behavior |
|---------|--------------|------------|----------------------|
| **Seamless cursor movement across screen edges** | This IS the product | High | Synergy/Barrier: absolute coord teleport at edge |
| **Keyboard input forwarding** | Core promise | High | All tools: capture on source, inject on sink. Modifier sync is the hard part |
| **Clipboard sharing (text)** | Most-requested feature after basic input | Medium | Synergy/Barrier: automatic clipboard sync. Unreliable for large content |
| **Multi-monitor support** | Power users always have multiple monitors | High | All tools: supported but layout config is painful. Edge-mapping bugs are endemic |
| **Auto-discovery of peers on LAN** | Users expect zero-config on same subnet | Medium | Barrier: no discovery (major complaint). Input Leap: added mDNS. Deskflow: mDNS |
| **Hotkey to release focus** | Escape hatch when cursor is stuck on wrong machine | Low | All tools: configurable hotkey (Scroll Lock or custom) |
| **Reconnection on network interruption** | WiFi drops, cable bumps | Medium | Barrier: notoriously bad, often requires restart. #1 user complaint |
| **Cross-platform key mapping (macOS ↔ Linux)** | Cmd-C on Mac should behave like Ctrl-C on Linux | High | All tools support this. Modifier remapping is fragile |
| **Low latency (≤20ms perceptible, ≤50ms tolerable)** | Users notice input lag; typing latency is especially obvious | High | Synergy/Barrier: <10ms on LAN typically. Nagle's algorithm must be disabled |
| **Daemon/service mode** | Start at boot, run in background | Medium | All tools: systemd on Linux, launchd on macOS |

## Differentiators

Features that make Periphore genuinely different.

| Feature | Value | Existing Tools |
|---------|-------|---------------|
| **Symmetric P2P peering (no server/client)** | Eliminates "which machine is the server?" — the most common setup complaint across all tools | All use client-server. Not one implements P2P |
| **Cryptographic identity (keypair + fingerprint)** | SSH-style TOFU model. Know which machine is connecting, not just that someone knows a password | Synergy: plaintext password. Barrier/Input Leap: optional TLS with painful self-signed cert setup that most users disable |
| **Identicon + word-phrase verification** | Two channels for fingerprint comparison — visual (identicon on both machines) and verbal (one reads, other types). No hex strings | No existing tool does this |
| **SSH-tunnelable by design (TCP-only)** | Power users already tunnel Barrier over SSH as a workaround; Periphore makes it first-class | Workaround in all other tools, not officially supported |
| **Config-file-only, no auto-writes** | Version-controllable config. Users who have been burned by Synergy's GUI overwriting their hand-edited config will notice this | Synergy GUI overwrites constantly. Barrier/Input Leap store config in opaque platform locations |
| **IPC socket for local control** | CLI tooling, status queries, future GUI as separate process, scripting | Synergy has proprietary IPC. Barrier has limited CLI. None expose a clean IPC |
| **Captive window mode (no accessibility permissions)** | Corporate environments often block Accessibility permission grants. Fullscreen capture avoids the requirement | All macOS tools require Accessibility permissions. No alternative offered |
| **Topology negotiation between peers** | Peers exchange monitor layouts and negotiate edge mappings. Eliminates most manual configuration | Manual arrangement only. No negotiation. Layout misconfiguration is the second-most-common support topic |

## Anti-Features

Explicitly OUT of scope — building these has burned every other tool.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **GUI configuration tool** | GUIs are the most fragile and platform-specific part of Synergy/Barrier. Barrier's Qt GUI is notoriously bug-prone | TOML config + CLI. IPC socket enables a future GUI as a separate binary |
| **Clipboard sharing of images/files** | Synergy's file drag-and-drop was buggy for years. Rich clipboard is complex and OS-specific | Text-only clipboard first. Design clipboard protocol to be MIME-extensible for later |
| **Drag-and-drop file transfer** | Enormously complex, OS-specific, fragile. Synergy file transfer: years of bugs | Suggest scp/rsync/shared folders. Consider post-1.0 extension channel |
| **Windows support** | Triples the platform matrix. Windows input APIs (SendInput, raw input hooks) are entirely different | macOS + Linux first. Design platform abstraction to accommodate Windows later |
| **Wayland compositor-specific support** | Each compositor has different extension protocols. Fragmentation makes this a swamp | Support X11. Captive window may work on Wayland without compositor-specific code. Track portal/extension APIs but don't commit |
| **Web/browser-based client** | WebRTC/WebSocket latency, limited input capture, new platform target | Defer entirely. Build as separate project using the Periphore protocol |
| **Audio forwarding** | Synergy attempted this; widely considered broken. Different latency requirements, codec needs, platform APIs | Design wire protocol with extensible channel types. Do not implement |
| **Automatic physical-position detection** | Unsolvable without user input | Auto-discover peers; require explicit edge relationship config |
| **Dynamic monitor hotplug** | Complex and race-prone | Re-read topology on reconnect or daemon restart. CLI-triggered rescan. Full hotplug deferred |
| **Gamepad/joystick forwarding** | Different input class, different latency requirements | Out of scope. Extension point for later |

## Feature Dependency Map

```
TCP Peer Connection
  ├── mDNS Auto-Discovery
  ├── Cryptographic Identity (keypair gen + fingerprint)
  │     ├── TOFU Verification Flow (identicon + word phrase)
  │     └── Fingerprint Caching (known_peers store)
  ├── Monitor Topology Exchange
  │     ├── Edge Mapping (per-monitor directional edges)
  │     └── Topology Negotiation (conflict resolution)
  ├── Input Capture — Captive Window Mode
  │     ├── Keyboard forwarding (cross-platform modifier mapping)
  │     ├── Mouse forwarding (relative movement)
  │     ├── Hotkey escape
  │     └── [Seamless mode — deferred]
  ├── Input Injection (sink side)
  ├── Clipboard (text) — defer past basic input
  └── IPC Socket
        └── CLI (status, topology debug, config validation)
```

## Competitive Security Analysis

| Tool | Authentication | Encryption | Identity Model | Rating |
|------|---------------|------------|----------------|--------|
| Synergy | Plaintext password | None → optional TLS | Shared secret | Poor |
| Barrier | Plaintext password | Optional TLS (painful) | Self-signed certs users ignore | Poor |
| Input Leap | Plaintext password | TLS (improved UX) | Auto-generated certs | Fair |
| Deskflow | Plaintext password | TLS | Similar to Input Leap | Fair |
| **Periphore** | Ed25519 keypair | TCP (SSH-tunnenable) | Cryptographic identity + TOFU | **Good** |

**None of the existing tools use public-key identity.** Periphore's security model is meaningfully better, not just marginally.

## What the Synergy/Barrier Community Complains About Most

1. Reconnection failures requiring manual restart (Barrier especially)
2. Modifier keys getting stuck after edge crossing
3. Multi-monitor edge misconfiguration (silent failures)
4. TLS certificate setup complexity (most users disable it)
5. macOS Accessibility permission requirement blocked in corporate environments
6. Config file overwritten by GUI
7. Latency spikes under CPU load
8. Clipboard sync dying and requiring restart
9. "Which machine is the server?" setup confusion
10. No headless/SSH-friendly operation

Periphore's design directly addresses 1, 4, 5, 6, 8, 9, and 10.

## MVP Build Order (Research Recommendation)

1. **TCP peering + cryptographic identity + IPC** — substrate
2. **mDNS discovery + manual host config** — discoverability
3. **Monitor topology query + peer exchange + edge negotiation** — layout
4. **Captive window input capture + keyboard/mouse injection** — core product value
5. **Hotkey escape + basic CLI via IPC** — usability
6. **Text clipboard** — table stakes completeness
7. **Reconnection with state recovery** — reliability
8. **Fingerprint caching between sessions** — security UX
