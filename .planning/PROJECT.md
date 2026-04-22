# Periphore

## What This Is

Periphore ("Peripheral Carrier") is a peer-to-peer input sharing daemon written in Rust. It enables keyboard and mouse control to flow between machines across screen edges — like Synergy or Barrier — but built on a symmetric source/sink model with no primary/secondary hierarchy, designed for extensibility beyond HID input (audio, future side-channels). macOS and Linux are the initial targets.

## Core Value

A machine's input devices should be able to reach any peer on the network, flowing naturally across screen edges, with verified identity and no central authority.

## Requirements

### Validated

(None yet — ship to validate)

### Active

**Peering & Transport**
- [ ] Two machines establish a peer connection over TCP
- [ ] Auto-discovery mechanism locates peers on the local network (Scenario 1/2)
- [ ] Manual host definition as alternative to auto-discovery (Scenario 2)
- [ ] Connections are SSH-tunnelable (TCP-only transport, no UDP)
- [ ] On Linux with X-Auth: service can be launched and supervised remotely via SSH
- [ ] On other systems: daemon must be pre-running; listens on IPC + TCP

**IPC Layer**
- [ ] Service exposes a Unix domain socket (platform-appropriate) for local IPC
- [ ] IPC enables modular peering layer and supports testing without a network peer

**Security & Identity**
- [ ] Each node generates a persistent keypair; fingerprint derived from public key
- [ ] Fingerprint displayed as an identicon (visual, shown on both machines)
- [ ] Fingerprint also available as a typed word phrase (one side reads, other types — not displayed on both simultaneously)
- [ ] Identicon display can be disabled (headless/automated setups)
- [ ] Accepted fingerprints cached between sessions (no auto-write to main config)
- [ ] Hard configuration can include fingerprint — conflicts prevent peering (Scenario 5)

**Monitor Topology**
- [ ] Service queries all monitors on the local machine
- [ ] Peer topology (monitor layout relative to each peer's monitors) configured via config file
- [ ] Peers exchange and negotiate topology on connection
- [ ] CLI debug output shows resolved topology when debug logging is enabled
- [ ] Support multi-monitor setups: machine with N monitors peers with machine with M monitors
- [ ] Corner resolution and offset compensation for mismatched monitor arrangements (Scenario 4)
- [ ] Edge definitions are directional per monitor (left/right/top/bottom of specific monitor) (Scenario 3)
- [ ] Smart cycling: system understands when edge traversal has looped back around (Scenario 3)

**Input Flow**
- [ ] Source machine captures input; sink machine injects it
- [ ] Bi-directional control (either machine can be source or sink) is optional per session
- [ ] Input is treated as relative (not absolute coordinates) for cross-machine movement
- [ ] Captive window mode: fullscreen/kiosk captures input without requiring accessibility permissions
- [ ] Hotkey escapes captive mode and returns input to local machine

**Configuration**
- [ ] No automatic config file writes; all config is explicit user-authored
- [ ] Hard config conflicts between peers prevent peering (Scenario 5)
- [ ] Config can define preferred monitor layouts for dynamic monitor scenarios

### Out of Scope

- Windows support — macOS and Linux only for v1
- GUI topology visualizer — deferred to a dedicated GUI phase
- Web client (Scenario 6) — browser-based control deferred
- Seamless edge-crossing via OS accessibility APIs (CGEvent tap / evdev) — deferred after captive window is stable
- Dynamic monitor add/remove detection and automatic re-layout (Scenario 8) — deferred
- USB peripheral pass-through, audio forwarding — future extension points only
- Wayland-specific input handling — not explicitly planned yet

## Context

- The project name is intentional: **Peripheral + -phore (to carry)** = "The Peripheral Carrier." Code and docs use **source** (machine sending input) and **sink** (machine receiving input) throughout.
- Synergy and Barrier are the competitive reference points. Periphore's differentiator is symmetric P2P peering — no dedicated server machine required.
- Phasing intent: establish the peering, topology, and security substrate first (no GUI, no input capture yet); then add captive window input; then add seamless accessibility-based input.
- The IPC layer (Unix domain socket) doubles as a modular boundary between the transport and capture layers, enabling unit testing without a live network peer.
- Rust was chosen for low-latency input handling and native cross-platform binary distribution.
- `package.json` is present solely for the GSD toolchain (`get-shit-done-cc`), not part of the Rust build.

## Constraints

- **Language**: Rust — chosen for performance and native binary distribution
- **Transport**: TCP only — no UDP; this is intentional to support SSH tunneling
- **Platforms**: macOS and Linux — Windows explicitly out of scope for now
- **Config discipline**: System never auto-writes configuration; all config is user-owned
- **Input capture order**: Captive window (no accessibility needed) → seamless (accessibility) — not concurrent development
- **GUI/WUI**: Explicitly deferred; v1 is headless daemon + CLI only

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| TCP-only transport | Enables SSH tunneling; simplifies connection model | — Pending |
| Unix domain socket for IPC | Modular peering boundary; enables testing without network | — Pending |
| Source/sink terminology over primary/secondary | Clearer directionality; avoids hierarchy implications | — Pending |
| Identicon + typed word phrase for fingerprinting | Identicon for visual comparison, word phrase for verbal/manual verification without screen exposure | — Pending |
| Captive window before seamless | Avoids accessibility permissions blocking early progress; simpler to implement and test | — Pending |
| Multi-monitor topology via config + CLI debug | Defers visual feedback to GUI phase while still supporting multi-monitor peering from day one | — Pending |
| P2P over client-server | No single machine needs to be "the server"; any machine can initiate | — Pending |
| Single branch (main only) | Commit history + conventional commits replace branching as audit/isolation mechanism; enforced via commitizen-branch in prek | — Pending |
| Conventional commits enforced via prek | Every commit is machine-readable; enables `cz bump`, `cz changelog`, and `git bisect` without manual tagging | — Pending |
| AI model attribution as best-effort footer | Committing agent records itself; multi-agent attribution is a future concern — don't over-engineer before the need is concrete | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-04-22 after initialization*
