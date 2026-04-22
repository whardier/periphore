# Periphore — Claude Code Guide

## Project

Periphore is a peer-to-peer input sharing daemon written in Rust. Keyboard and mouse control flows between machines across screen edges — like Synergy/Barrier — but built on a symmetric source/sink model with no primary/secondary hierarchy. macOS and Linux are the initial targets.

**Key terminology:** "source" = machine sending input, "sink" = machine receiving input.

## Technology Stack

- **Language:** Rust (performance, native binary distribution)
- **Runtime:** Tokio (async TCP + Unix domain sockets)
- **Protocol framing:** `tokio-util` `LengthDelimitedCodec` + `postcard`/`serde`
- **Transport:** TCP only — no UDP (enables SSH tunneling)
- **Input capture:** `rdev` for captive-window phase
- **Crypto:** `ed25519-dalek`, `sha2`
- **Config:** `clap` v4 + `figment` + TOML
- **Discovery:** `mdns-sd`
- **Logging:** `tracing` + `tracing-subscriber`

## GSD Workflow

This project uses GSD (Get Shit Done) for structured execution.

**Planning artifacts are in `.planning/`:**
- `PROJECT.md` — requirements, constraints, key decisions
- `REQUIREMENTS.md` — 30 REQ-IDs across 6 categories
- `ROADMAP.md` — 10-phase execution plan
- `STATE.md` — current phase and progress
- `research/` — domain research (stack, features, architecture, pitfalls)

**GSD commands:**
- `/gsd:next` — detect state and advance to next step automatically
- `/gsd:discuss-phase N` — gather context for phase N
- `/gsd:plan-phase N` — create execution plan for phase N
- `/gsd:execute-phase N` — execute plans for phase N
- `/gsd:verify-work` — verify phase deliverables against success criteria
- `/gsd:progress` — show current state

## Constraints

- **Config discipline:** The system NEVER auto-writes configuration files. All config is user-authored.
- **Input capture order:** Captive window (no accessibility needed) first, then seamless (accessibility) — not concurrent.
- **No GUI:** v1 is headless daemon + CLI only. GUI is explicitly deferred.
- **Platforms:** macOS and Linux only. Windows is out of scope.
- **Commits:** Conventional commits enforced via commitizen. Single branch (main).

## Architecture

Cargo workspace with purpose-scoped crates:
- `periphore-protocol` — shared message types (compile-time foundation)
- `periphore-config` — layered config loading (never writes to disk)
- `periphore-identity` — Ed25519 keypairs, fingerprints, identicons, word phrases
- `periphore-core` — pure-logic state machine (zero platform deps, fully testable)
- `periphore-ipc` — Unix domain socket service boundary
- `periphore-cli` — CLI crate; produces the `periphore` command (not `periphore-cli`)
- `periphore-net` — TCP peering, handshake, topology negotiation
- `periphore-capture` — platform input capture (rdev → direct evdev/CGEvent)
- `periphore-inject` — platform input injection
- `periphore` — daemon binary crate

**Build order:** protocol → config + identity → core + ipc + cli → net → capture + inject

## Critical Implementation Notes

1. **`TCP_NODELAY` must be set immediately** — Nagle's algorithm causes 40–200ms latency spikes
2. **macOS Secure Input silently disables CGEvent taps** — captive window avoids this entirely
3. **evdev requires udev rules** — ship `99-periphore.rules`; never run as root
4. **Modifier key desync on edge crossing** — flush all held modifiers on `FocusTransfer`
5. **Mouse-move coalescing** — bounded channels + drop stale moves; never coalesce key/button events
6. **mDNS fails silently on corporate networks** — manual host config must always work as fallback
