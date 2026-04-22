# Stack Research — Periphore

## Runtime: Tokio

**Choice:** `tokio` (latest stable, currently 1.x)

The undisputed async runtime for production Rust daemons. Provides async TCP, Unix domain sockets, timers, signal handling, and process management. No realistic alternative for a networked daemon.

- `tokio` — async runtime, TCP/Unix sockets, signal handling
- `tokio-util` — codec framing (`LengthDelimitedCodec`, `Framed`) for the peer protocol
- `bytes` — `Bytes`/`BytesMut` for zero-copy buffer management

**Protocol framing:** Use `tokio_util::codec::LengthDelimitedCodec` with a 4-byte big-endian length header. Pair with `serde` + `postcard` (compact binary, `#![no_std]`-compatible) or `bincode` for message serialization. Postcard is preferred for tightly constrained binary formats; bincode is simpler.

---

## Input Capture

### macOS

No pure-Rust solution that avoids CGEvent FFI. Options:

| Crate | Approach | Notes |
|-------|----------|-------|
| `rdev` | CGEvent tap via FFI, supports grab+listen | Needs Accessibility + Input Monitoring permissions; actively maintained; supports macOS, Linux, Windows |
| Custom CGEvent FFI | Direct `core-foundation` + `core-graphics` | More control; same permission requirements; more work |

**Recommendation:** Start with `rdev` for the captive-window phase (it provides a simpler API and handles platform differences). For the seamless edge-detection phase, evaluate whether direct CGEvent FFI gives needed control.

**Critical caveat:** macOS disables event taps silently under Secure Input (password fields, sudo prompts). No callback or error is fired. Must poll `CGEventTapIsEnabled` continuously. See PITFALLS.md.

### Linux

| Crate | Approach | Notes |
|-------|----------|-------|
| `evdev` | Pure Rust libevdev re-implementation | Read events from `/dev/input/event*`; also supports uinput for injection |
| `evdev-rs` | Bindings to C libevdev | Alternative if `evdev` pure-Rust has gaps |
| `rdev` | Uses evdev under the hood, grab support | Works with X11 and Wayland for listening; injection harder on Wayland |

**Recommendation:** Use `evdev` directly for Linux — gives raw access to devices and uinput for injection. Avoid `rdev` on Linux for capture; use it only if a unified API is needed.

**Wayland note:** evdev capture works (reads from `/dev/input/event*` directly), but uinput injection may not reach Wayland compositors without portal/protocol support. X11 remains the reliable injection target for now.

---

## Input Injection

### macOS

`CGEventPost` via `core-graphics` or `rdev::simulate`. The `rdev` crate provides `simulate(EventType)` which wraps `CGEventPost` appropriately.

### Linux

`evdev` crate's uinput support (`evdev::uinput::VirtualDevice`). Creates a virtual kernel input device. Requires write access to `/dev/uinput` — handled via udev rule (see PITFALLS.md).

---

## IPC (Unix Domain Socket)

`tokio::net::UnixListener` / `tokio::net::UnixStream` — built into Tokio. No additional crate needed.

Use a path under `$XDG_RUNTIME_DIR` on Linux (e.g., `/run/user/1000/periphore.sock`) and `$TMPDIR` on macOS. Runtime directory selection should use the `dirs` or `directories` crate.

---

## Service Discovery (mDNS)

**Choice:** `mdns-sd` (181k downloads/month, 136 downstream crates, actively maintained)

- Pure Rust, no system Bonjour/Avahi dependency
- Spawns its own thread internally; API uses `flume` channels — works cleanly with async code
- Supports both querier (browse) and responder (announce) roles
- Cross-platform: macOS, Linux, Windows

**Alternative:** `zeroconf` — wraps system Bonjour/Avahi. Avoid: adds system dependency and Bonjour (the legacy auto-discovery used in Synergy) was removed from Synergy due to stability issues.

---

## Cryptography

| Component | Crate | Notes |
|-----------|-------|-------|
| Keypair generation | `ed25519-dalek` | Dalek-cryptography suite; well-audited; note the GitHub repo is archived/moved to the `curve25519-dalek` workspace — use the crates.io release |
| CSPRNG | `rand` + `getrandom` | For keypair generation seed |
| Fingerprint hash | `sha2` (RustCrypto) | SHA-256 of the public key bytes |
| Identicon rendering | `identicon-rs` or custom | Deterministic blocky pattern from hash; verify determinism across platforms before shipping |
| Word-phrase | Custom BIP39-inspired wordlist | Or `bip39` crate; 4-6 words from SHA-256 hash of pubkey |

**Note on ed25519-dalek:** The crate moved homes. Pin to a known-good version from crates.io. The API surface is stable.

---

## Configuration

**Pattern:** Layered config via `figment`

```
Defaults (compiled-in) < TOML file < Environment variables < CLI flags
```

| Crate | Role |
|-------|------|
| `clap` v4 | CLI argument parsing (derive API) |
| `figment` | Layered config merging (file + env + CLI) |
| `toml` | TOML deserialization |
| `serde` | Config struct serialization |

**Note:** Figment's default priority order is inverted from what you want. Must explicitly chain providers in the right order: `Figment::new().merge(defaults).merge(toml).merge(env).merge(cli)`.

**Fingerprint cache:** Separate from the main config. Store in XDG cache dir (`$XDG_CACHE_HOME/periphore/known_hosts.toml`). Never written by the main config path — only by the trust acceptance flow.

---

## Daemon Lifecycle

### Linux

`systemd` is the standard. Use `sd-notify` crate to send `READY=1` after initialization. Unit file uses `Type=notify`.

### macOS

LaunchDaemon or LaunchAgent plist. No Rust crate needed — the plist is a static file installed by the user.

For both: handle `SIGTERM` and `SIGHUP` via `tokio::signal`.

---

## Monitor Enumeration

### macOS

`core-graphics` crate — provides `CGDisplay::active_displays()`. Returns display IDs; from each ID you can get bounds, resolution, and identifier.

### Linux (X11)

`x11rb` crate with the `randr` extension. Queries `RRGetMonitors` for monitor list with geometry.

### Linux (Wayland / framebuffer)

No clean Rust crate yet. Fallback: parse `/sys/class/drm/*/edid` and geometry from `/sys/class/drm/*/modes`, or shell out to `wlr-randr` for compositors that support it.

**Recommendation:** Implement an abstraction trait `MonitorProvider` with a macOS implementation (CoreGraphics) and a Linux X11 implementation (x11rb/randr). Wayland support is best-effort for now.

---

## Serialization

**Wire protocol:** `postcard` — compact, deterministic, works with `serde`. Good for fixed-schema binary protocol messages.

**Config files:** `toml` — human-readable, user-editable.

**Logging:** `tracing` + `tracing-subscriber` — structured, async-aware. Debug topology output via `tracing::debug!`.

---

## Summary Table

| Component | Crate(s) | Confidence |
|-----------|----------|------------|
| Async runtime | `tokio` | High |
| Protocol framing | `tokio-util` + `bytes` | High |
| Wire serialization | `postcard` + `serde` | High |
| Unix IPC | `tokio::net::UnixListener` | High |
| Input capture (macOS) | `rdev` → custom CGEvent FFI later | Medium |
| Input capture (Linux) | `evdev` | High |
| Input injection (macOS) | `rdev::simulate` | Medium |
| Input injection (Linux) | `evdev` uinput | High |
| mDNS discovery | `mdns-sd` | High |
| Keypair | `ed25519-dalek` | High |
| Fingerprint | `sha2` | High |
| Identicon | `identicon-rs` or custom | Low — verify determinism |
| Config layering | `figment` + `clap` + `toml` | High |
| Monitor enum (macOS) | `core-graphics` | High |
| Monitor enum (Linux) | `x11rb` (randr) | Medium |
| Daemon signals | `tokio::signal` | High |
| Logging/tracing | `tracing` + `tracing-subscriber` | High |
| Path dirs | `directories` | High |
