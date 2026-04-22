# Research Summary — Periphore

**Researched:** 2026-04-22

---

## Stack

**Runtime:** Tokio (TCP + Unix socket via built-in `tokio::net`). No alternative.

**Protocol framing:** `tokio-util` `LengthDelimitedCodec` (4-byte big-endian length prefix) + `postcard`/`serde` for binary serialization.

**Input capture:** `rdev` (cross-platform, wraps CGEventTap/evdev) for captive-window phase. Upgrade to direct `evdev` crate on Linux and CGEventTap FFI on macOS for seamless phase.

**Input injection:** `rdev::simulate` (macOS), `evdev` uinput (`VirtualDevice`) on Linux.

**mDNS discovery:** `mdns-sd` — most popular (181k downloads/month), pure Rust, works with Tokio.

**Crypto:** `ed25519-dalek` for keypairs; `sha2` for fingerprint hash; identicon from hash (verify determinism across platforms); word-phrase from BIP39-inspired wordlist.

**Config:** `clap` v4 + `figment` + `toml`. Layered: defaults < file < env < CLI.

**Monitor enumeration:** `core-graphics` (macOS), `x11rb` with randr extension (Linux X11). Wayland: best-effort via `/sys/class/drm` or `wlr-randr`.

**Logging:** `tracing` + `tracing-subscriber`.

---

## Table Stakes

- Seamless cursor edge crossing (or captive-window equivalent)
- Keyboard forwarding with cross-platform modifier mapping
- Text clipboard sharing (defer past v1 service layer)
- Multi-monitor support
- mDNS auto-discovery + manual host config fallback
- Hotkey to release input focus
- Reconnection without restart
- Daemon/service mode

---

## Differentiators

- **P2P symmetric peering** — eliminates server/client designation entirely; no existing tool does this
- **Cryptographic identity + TOFU** — identicon + word-phrase fingerprint verification; all competitors use plaintext passwords or broken TLS UX
- **SSH-tunnelable by design** — TCP-only constraint is a feature
- **Config-file-only, no auto-writes** — version-controllable, user-owned config
- **IPC socket** — enables CLI tooling, debugging, future GUI as separate process
- **Captive window (no accessibility permissions)** — works in corporate environments where Accessibility is blocked

---

## Watch Out For

1. **TCP `TCP_NODELAY` must be set immediately** — Nagle's algorithm creates 40–200ms latency spikes; non-negotiable
2. **macOS Secure Input silently disables CGEvent taps** — captive window avoids this entirely; validates phasing decision
3. **evdev requires udev rules** — ship `/etc/udev/rules.d/99-periphore.rules`; never run as root
4. **Modifier key desync on edge crossing** — flush all held modifiers on `FocusTransfer`
5. **Mouse-move coalescing** — bounded channels + drop stale moves; do not coalesce key/button events
6. **mDNS fails silently on corporate networks** — manual host config must always work as fallback

---

## Architecture Recommendation

**Cargo workspace** with `periphore-protocol` (shared types), `periphore-core` (pure-logic state machine, zero platform deps), `periphore-net`, `periphore-ipc`, `periphore-capture`, `periphore-inject`, `periphore-config`, `periphore-identity`, `periphore-cli` (CLI binary, command: `periphore`).

**Channel-based concurrency** (`tokio::mpsc`, bounded). Each component is an isolated Tokio task communicating via typed channels. The state machine in `periphore-core` is purely functional (input → output actions) — testable without any platform code or network.

**Build order:** protocol → config + identity → core + ipc + cli → net → capture + inject.

---

## Phase Recommendation

| Phase | Scope |
|-------|-------|
| 1 | `periphore-protocol`, `periphore-config`, `periphore-identity` |
| 2 | `periphore-core` (state machine), `periphore-ipc`, `periphore-cli` |
| 3 | `periphore-net` (TCP, peer handshake, topology negotiation) |
| 4 | `periphore-capture` + `periphore-inject` (captive window mode) |
| 5+ | Seamless capture, clipboard, reconnection polish, GUI (separate binary) |
