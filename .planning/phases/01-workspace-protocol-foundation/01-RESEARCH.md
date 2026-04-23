# Phase 1: Workspace & Protocol Foundation — Research

**Researched:** 2026-04-22
**Domain:** Cargo workspace structure, Rust protocol serialization, Tokio Unix IPC, Figment config layering
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Workspace Scaffold
- **D-01:** All 11 crates scaffolded in Phase 1 — `crates/` flat layout following uv/typst pattern (`members = ["crates/*"]`)
- **D-02:** `default-members = ["crates/periphored", "crates/periphore"]` so plain `cargo build` builds both binaries
- **D-03:** Every crate declared in `[workspace.dependencies]` with both `path` and `version` — no bare path refs inside individual crate Cargo.tomls
- **D-04:** `[workspace.lints.rust]` + `[workspace.lints.clippy]` set from day one; all crates use `[lints] workspace = true`
- **D-05:** Binary crates live inside `crates/`, not at workspace root — `crates/periphored/src/main.rs` (daemon), `crates/periphore/src/main.rs` (CLI entry); `crates/periphore-cli/` is a library (no `main`)
- **D-06:** Feature gating via features on internal crates (e.g., `periphore-config` gets a `clap` feature activated only by `periphore-cli`)
- **D-07:** `[lib] doctest = false test = false` on thin foundational crates (`periphore-protocol`, `periphore-identity`)

#### Crates Implemented vs Stubbed
- **D-08:** Actively implemented in Phase 1: `periphore-protocol`, `periphore-config`, `periphore-ipc`, `crates/periphored` (daemon binary entry)
- **D-09:** Stubbed for later phases: `periphore-identity` (Phase 2), `periphore-core` (Phase 4+), `periphore-net` (Phase 6), `periphore-capture` (Phase 9), `periphore-inject` (Phase 9)
- **D-10:** `crates/periphore` (thin CLI entry) and `crates/periphore-cli` (CLI library) both scaffolded as stubs — full implementation in Phase 5

#### Protocol Types (`periphore-protocol`)
- **D-11:** Full PeerMessage enum defined — all ~15 variants: Hello, HelloAck, TopologyAdvertise, TopologyPropose, TopologyAccept, TopologyReject, FocusTransfer, FocusAck, FocusReclaim, MouseMove, MouseButton, MouseScroll, KeyEvent, Ping, Pong, Bye
- **D-12:** Full supporting type surface defined: `MonitorInfo`, `Edge` (Left/Right/Top/Bottom), `EdgeMapping`, `InputEvent` (Mouse/Key), `MouseEvent`, `KeyEvent`
- **D-13:** Serialization: `serde` + `postcard`; framing: `tokio-util LengthDelimitedCodec` (4-byte big-endian length header)
- **D-14:** Round-trip tests for the full PeerMessage enum in the protocol crate

#### IPC Layer (`periphore-ipc`)
- **D-15:** Full IpcRequest enum implemented: `GetStatus`, `ListPeers`, `GetTopology`, `AcceptFingerprint`, `RejectFingerprint`, `ReloadConfig`, `InjectInputEvent { event: InputEvent }`, `SimulateEdgeCross { edge: Edge, position: f64 }`, `GetState`, `GetPendingVerifications`, `GetIdenticon`, `GetWordPhrase`
- **D-16:** Protocol: JSON-lines over Unix domain socket (newline-delimited JSON) — local-only so human-readable is fine
- **D-17:** Socket path: Linux `$XDG_RUNTIME_DIR/periphore/periphore.sock`, macOS `$TMPDIR/periphore/periphore.sock`; permissions `0600`
- **D-18:** Daemon creates socket on startup, removes on clean shutdown (including SIGTERM/SIGHUP via `tokio::signal`)
- **D-19:** IPC implemented as the testing backbone — `InjectInputEvent` and `SimulateEdgeCross` must be exercisable from Phase 1 forward
- **D-20:** Phase 4 (IPC Layer) is retained in the roadmap for IPC enhancements, not removed

#### Config Schema (`periphore-config`)
- **D-21:** Full schema defined upfront — all top-level sections present: `[daemon]`, `[logging]`, `[[peer]]`, `[topology]`, `[capture]`
- **D-22:** Layering order: `Figment::new().merge(defaults).merge(toml).merge(env).merge(cli)` — Figment's default order is inverted from what's needed, must be explicit
- **D-23:** Config never writes to disk under any code path — enforced at compile time (no `Serialize` impl on the config struct, no write paths)
- **D-24:** Fingerprint cache is separate from main config (stored in XDG cache dir, written only by trust acceptance flow) — config crate does not own this path
- **D-25:** `clap` feature on `periphore-config` gates the CLI-integrated config struct (only activated by `periphore-cli` library, which is only pulled in by the `periphore` CLI binary)

#### Daemon Binary (`crates/periphored`)
- **D-26:** Thin `src/main.rs` — no business logic; wires together `periphore-ipc`, `periphore-config`, and other functional crates via channels
- **D-27:** Daemon starts IPC socket, responds to `GetStatus` with identity fingerprint placeholder and running status
- **D-28:** `periphored --help` produces usage output via `clap` v4 derive API
- **D-29:** Signal handling: SIGTERM and SIGHUP handled via `tokio::signal`; clean shutdown removes IPC socket

#### CLI Binary Entry (`crates/periphore`)
- **D-30:** Thin `src/main.rs` — calls into `periphore-cli` library for all command dispatch and IPC client logic
- **D-31:** `periphore --help` produces usage output; all subcommand implementations live in `periphore-cli`

### Claude's Discretion
- Exact Clippy lint configuration (which pedantic lints to allow vs warn) — follow uv's pattern of `pedantic = warn` with selective overrides
- Whether to add `resolver = "2"` and `edition = "2024"` at workspace level (yes — both reference projects use this)
- Workspace package metadata fields (`homepage`, `repository`, `license`) — reasonable defaults are fine
- Whether `periphore-protocol` re-exports its types from a top-level `lib.rs` or uses modules — Claude decides

### Deferred Ideas (OUT OF SCOPE)
- Full IPC command richness — additional IPC commands beyond the Phase 1 set belong in Phase 4 enhancements
- `periphore-cli` real implementation — Phase 5
- Identity/fingerprint types in protocol — Phase 2 fills these in (stubs in Phase 1)
- Platform-specific capture/inject — Phases 9–10
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CFG-01 | System never auto-writes configuration; all config is user-authored | Figment layering with `Serialized::defaults` + `Toml::file` + `Env::prefixed` + CLI; config struct intentionally has no `Serialize` impl; no write paths in crate |
| IPC-01 | Service exposes a Unix domain socket (platform-appropriate) for local IPC | `tokio::net::UnixListener` at `$XDG_RUNTIME_DIR/periphore/periphore.sock` (Linux) or `$TMPDIR/periphore/periphore.sock` (macOS) with `0600` permissions |
| IPC-02 | IPC layer is the modular boundary between transport and capture, testable without a network peer | `InjectInputEvent` and `SimulateEdgeCross` in `IpcRequest` enum; daemon can run with net/capture disabled; state observable via `GetState` |
</phase_requirements>

---

## Summary

Phase 1 establishes the entire Cargo workspace scaffold — all 11 crates, the workspace dependency graph, workspace-level lints, and two working binary targets — before implementing three functional areas: the wire protocol type surface, the layered config system, and the IPC socket backbone. Every subsequent phase depends on these foundations being correct from day one, making this the highest-leverage phase in the project.

The technical work splits cleanly into four tracks. The workspace scaffold track is pure Cargo configuration: `members = ["crates/*"]`, `default-members`, `[workspace.dependencies]` with path + version for all 11 crates, and `[workspace.lints]`. The protocol track implements the full `PeerMessage` enum with `serde` + `postcard` serialization and `tokio-util LengthDelimitedCodec` framing — all types must round-trip correctly since later phases build directly on this wire format. The config track uses Figment's `Serialized::defaults` → `Toml::file` → `Env::prefixed` → CLI merge chain, with the critical invariant that the config struct never gets `Serialize` (enforcing the no-auto-write rule at compile time). The IPC track implements the Unix domain socket server with JSON-lines protocol and the full `IpcRequest` enum, which immediately becomes the test harness for all future phases.

The primary risk in this phase is configuration: Figment's `.merge()` semantics (incoming wins) are correct but require explicit layering order — the last `.merge()` call wins, so CLI args must be the final merge. A secondary risk is the `LengthDelimitedCodec` default configuration: it uses a 4-byte big-endian length header by default, which is correct for this project, but the builder API must be used explicitly to confirm this rather than relying on `.new()` defaults if there is any doubt.

**Primary recommendation:** Scaffold all 11 crates in the first task, establish the workspace dependency graph, then implement protocol types, config, and IPC in parallel tracks — each independently verifiable via unit tests before the daemon wires them together.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Workspace scaffold (Cargo.toml, crate stubs) | Build system | — | Pure configuration, no runtime tier |
| Wire protocol types (`PeerMessage`, supporting types) | `periphore-protocol` crate | `periphore-net` (Phase 6, consumer) | Shared vocabulary; zero runtime deps in this crate |
| Wire serialization (postcard, LengthDelimitedCodec) | `periphore-protocol` crate | `periphore-net` (codec wrapper) | Framing belongs with type definitions |
| Layered config loading | `periphore-config` crate | `periphored` (consumer via figment extract) | Config is read-only from all consumers |
| IPC socket server | `periphore-ipc` crate | `periphored` (host, channels) | IPC is a functional crate; daemon wires it |
| IPC request dispatch | `periphored` `main.rs` | `periphore-ipc` (socket layer only) | Daemon owns routing; IPC owns transport |
| Platform socket path resolution | `periphore-ipc` crate | `directories` crate (path lookup) | Socket path is IPC's concern |
| Signal handling (SIGTERM, SIGHUP) | `periphored` `main.rs` | `tokio::signal` | Signal handling is daemon lifecycle, not a functional crate |
| CLI arg parsing (daemon) | `periphored` `main.rs` | `clap` v4 derive | Thin binary, clap generates --help |
| CLI arg parsing (periphore binary) | `periphore-cli` library | `periphore` thin entry | Command dispatch lives in library, not binary |

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tokio` | 1.52.1 | Async runtime, UnixListener, signal handling | Undisputed production async runtime for Rust daemons |
| `tokio-util` | 0.7.18 | `LengthDelimitedCodec`, `Framed` stream | Standard codec framing for TCP streams; pairs with tokio |
| `serde` | 1.0.228 | Derive macros for `Serialize`/`Deserialize` | Universal Rust serialization framework |
| `postcard` | 1.1.3 | Compact binary serialization for `PeerMessage` | `#![no_std]`-compatible, compact varints, deterministic; superior to bincode for wire protocols |
| `serde_json` | 1.0.149 | JSON-lines serialization for IPC protocol | Standard JSON crate; IPC is human-readable JSON-lines |
| `figment` | 0.10.19 | Layered configuration from defaults + TOML + env + CLI | Correct layering semantics; production-proven in Rocket |
| `clap` | 4.6.1 | CLI argument parsing with derive API | v4 derive is the idiomatic Rust CLI pattern |
| `tracing` | 0.1.44 | Structured async-aware logging | Required by CLAUDE.md; standard for Tokio applications |
| `tracing-subscriber` | 0.3.23 | Log output formatting and filtering | Pairs with tracing; provides env-filter |
| `bytes` | 1.11.1 | `Bytes`/`BytesMut` for codec buffer management | Required by `tokio-util` codec API |
| `directories` | 6.0.0 | Platform-specific socket/config paths | Correct XDG and macOS `$TMPDIR` resolution |
| `thiserror` | 2.0.18 | Ergonomic error type derivation | Standard for library crate errors |

### Supporting (Phase 1 stubs only)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `ed25519-dalek` | 2.2.0 | Ed25519 keypair (Phase 2 identity) | Stub dep in `periphore-identity`; implemented Phase 2 |
| `sha2` | 0.10.9 | SHA-256 fingerprint derivation (Phase 2) | Stub dep in `periphore-identity`; implemented Phase 2 |
| `tokio-stream` | — | `UnixListenerStream` for IPC connection iteration | Optional; direct `loop { listener.accept().await }` works |

**Note on ed25519-dalek version:** The latest crates.io entry for `ed25519-dalek` is `3.0.0-pre.6` (pre-release only). The last stable release is `2.2.0`. [VERIFIED: crates.io API] Use `2.2.0` in `[workspace.dependencies]` until 3.x stabilizes.

**Note on sha2 version:** `0.11.0` is the latest on crates.io but was a release candidate (`rc`) series in cargo search output. The last confirmed stable (non-rc) is `0.10.9`. [VERIFIED: crates.io API] Use `0.10.9`.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `postcard` | `bincode` | bincode is simpler but less compact; not `no_std`-compatible; postcard is the better wire protocol choice |
| `postcard` | `prost` (protobuf) | protobuf requires schema files; postcard uses serde derive directly; protobuf overkill for this project |
| `figment` | `config-rs` | config-rs is heavier and less composable; figment has cleaner layering API |
| `figment` | `envy` + manual toml | Manual composition loses figment's error provenance tracking |
| `directories` | Manual env var reads | `$XDG_RUNTIME_DIR` and `$TMPDIR` fallback logic is non-trivial; directories crate handles it correctly |
| `thiserror` | `anyhow` | Library crates should use `thiserror` (typed errors); only binary entry points should use `anyhow` |

**Installation (workspace root):**
```bash
# These are added to [workspace.dependencies], not installed globally
# Individual crate Cargo.tomls reference them as { workspace = true }
```

**Version verification:** All versions above verified via `cargo search` and `crates.io` API on 2026-04-22. [VERIFIED: cargo registry]

---

## Architecture Patterns

### System Architecture Diagram

```
                   +------------------+
                   |  User / Process  |
                   +--------+---------+
                            |
                    SIGTERM/SIGHUP     periphore --help
                            |                |
              +-------------+------+  +------+-----------+
              | periphored main.rs |  | periphore main.rs |
              |  (daemon entry)    |  |  (CLI stub entry) |
              +----+---+----+------+  +-------------------+
                   |   |    |               (calls periphore-cli stub)
          config   |   |    | ipc_cmd_tx
          load     |   |    |
                   |   |    v
    +--------------+   |  +------------------+
    |periphore-config|  |  | periphore-ipc    |
    |  (Figment      |  |  | UnixListener     |
    |   layering)    |  |  | JSON-lines       |
    +----------------+  |  | IpcRequest enum  |
                        |  +---------+--------+
                        |            |
                        |    IpcRequest::GetStatus / InjectInputEvent / etc.
                        |            |
                   tokio::select!    |
                        |            |
                   +----+------------+----+
                   |   router (Phase 4+) |
                   |   (placeholder now) |
                   +---------------------+

    periphore-protocol (pure type crate, no runtime deps):
    PeerMessage (serde + postcard) + IpcRequest/IpcResponse (serde + serde_json)
    MonitorInfo, Edge, EdgeMapping, InputEvent, MouseEvent, KeyEvent
```

Data flow for Phase 1 IPC test:
1. Test client opens Unix socket at platform path
2. Sends JSON-lines `IpcRequest::GetStatus\n`
3. `periphore-ipc` deserializes, sends `IpcCommand::GetStatus` to daemon's `ipc_cmd_rx`
4. Daemon's `tokio::select!` loop handles command, returns `IpcResponse::Status { running: true, ... }`
5. `periphore-ipc` serializes response, writes JSON-lines back to client

### Recommended Project Structure
```
Cargo.toml                          (workspace root)
Cargo.lock
.gitignore
crates/
  periphore-protocol/
    Cargo.toml
    src/lib.rs                      (re-exports all types)
    src/peer.rs                     (PeerMessage enum)
    src/ipc.rs                      (IpcRequest, IpcResponse enums)
    src/types.rs                    (MonitorInfo, Edge, EdgeMapping, InputEvent)
  periphore-config/
    Cargo.toml
    src/lib.rs                      (Config struct, load() fn)
    src/schema.rs                   (full schema: Daemon, Logging, Peer, Topology, Capture)
  periphore-ipc/
    Cargo.toml
    src/lib.rs                      (pub fn serve(path, tx))
    src/server.rs                   (UnixListener accept loop)
    src/path.rs                     (platform socket path resolution)
  periphore-identity/
    Cargo.toml
    src/lib.rs                      (stub — Phase 2)
  periphore-core/
    Cargo.toml
    src/lib.rs                      (stub — Phase 4+)
  periphore-net/
    Cargo.toml
    src/lib.rs                      (stub — Phase 6)
  periphore-capture/
    Cargo.toml
    src/lib.rs                      (stub — Phase 9)
  periphore-inject/
    Cargo.toml
    src/lib.rs                      (stub — Phase 9)
  periphore-cli/
    Cargo.toml
    src/lib.rs                      (stub — Phase 5)
  periphore/
    Cargo.toml
    src/main.rs                     (thin: calls periphore_cli::main() stub)
  periphored/
    Cargo.toml
    src/main.rs                     (daemon entry: config + ipc + signal + tokio::select!)
```

### Pattern 1: Workspace Cargo.toml Structure

**What:** Canonical workspace root configuration for an 11-crate Rust project.
**When to use:** Always — this is the single source of truth for all dependency versions and lint policy.

```toml
# Source: astral-sh/uv, typst/typst (verified against production Rust multi-crate projects)
[workspace]
resolver = "2"
members = ["crates/*"]
default-members = ["crates/periphored", "crates/periphore"]

[workspace.package]
version = "0.1.0"
edition = "2024"
authors = ["Periphore Contributors"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/whardier/periphore"
homepage = "https://github.com/whardier/periphore"
publish = false

[workspace.dependencies]
# Internal crates — path + version, referenced as { workspace = true } everywhere
periphore-protocol = { path = "crates/periphore-protocol", version = "0.1.0" }
periphore-config   = { path = "crates/periphore-config",   version = "0.1.0" }
periphore-identity = { path = "crates/periphore-identity", version = "0.1.0" }
periphore-core     = { path = "crates/periphore-core",     version = "0.1.0" }
periphore-ipc      = { path = "crates/periphore-ipc",      version = "0.1.0" }
periphore-net      = { path = "crates/periphore-net",      version = "0.1.0" }
periphore-capture  = { path = "crates/periphore-capture",  version = "0.1.0" }
periphore-inject   = { path = "crates/periphore-inject",   version = "0.1.0" }
periphore-cli      = { path = "crates/periphore-cli",      version = "0.1.0" }

# External dependencies
tokio        = { version = "1.52", features = ["net", "macros", "rt-multi-thread", "signal", "io-util", "sync", "time"] }
tokio-util   = { version = "0.7", features = ["codec"] }
serde        = { version = "1.0", features = ["derive"] }
serde_json   = { version = "1.0" }
postcard     = { version = "1.1", features = ["alloc"] }
bytes        = { version = "1.11" }
figment      = { version = "0.10", features = ["toml", "env"] }
clap         = { version = "4.6", features = ["derive"] }
tracing      = { version = "0.1" }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
directories  = { version = "6.0" }
thiserror    = { version = "2.0" }
ed25519-dalek = { version = "2.2" }
sha2         = { version = "0.10" }

[workspace.lints.rust]
unsafe_code    = "warn"
unreachable_pub = "warn"

[workspace.lints.clippy]
pedantic = "warn"
# Selective overrides for pedantic false positives:
module_name_repetitions = "allow"
missing_errors_doc      = "allow"
missing_panics_doc      = "allow"
```

### Pattern 2: Individual Crate Cargo.toml

**What:** Minimal per-crate Cargo.toml that delegates everything to the workspace.
**When to use:** Every crate in the workspace.

```toml
# Source: astral-sh/uv pattern (verified)
[package]
name = "periphore-protocol"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
publish.workspace = true

# On thin/foundational crates only:
[lib]
doctest = false
test = false

[lints]
workspace = true

[dependencies]
serde    = { workspace = true }
postcard = { workspace = true }
```

### Pattern 3: Figment Layered Config

**What:** Correct Figment merge order — each `.merge()` call adds a higher-priority source. Incoming wins.
**When to use:** `periphore-config` `load()` function.

```rust
// Source: Context7 /sergiobenitez/figment (verified)
use figment::{Figment, providers::{Format, Toml, Env, Serialized}};
use serde::Deserialize;
// NOTE: Config intentionally does NOT derive Serialize — enforces no-auto-write (D-23)

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub daemon:   DaemonConfig,
    pub logging:  LoggingConfig,
    pub peers:    Vec<PeerConfig>,
    pub topology: TopologyConfig,
    pub capture:  CaptureConfig,
}

impl Config {
    pub fn load(config_path: Option<&std::path::Path>) -> Result<Self, figment::Error> {
        let mut figment = Figment::from(Serialized::defaults(Config::default()));

        if let Some(path) = config_path {
            figment = figment.merge(Toml::file(path));
        }

        figment = figment
            .merge(Env::prefixed("PERIPHORE_").split("_"));
        // CLI args merged last by caller (D-22)

        figment.extract()
    }
}
```

**Critical:** `merge()` means "incoming wins over existing." So `Serialized::defaults` first (lowest priority), TOML second, env third, CLI last (highest priority). This is correct — but easy to reverse accidentally.

**Critical:** The `clap` feature on `periphore-config` gates a separate `CliOverrides` struct that a CLI consumer can merge in last. The `periphore-config` crate does NOT call `clap::parse()` itself.

### Pattern 4: postcard Round-Trip for PeerMessage

**What:** Serialize a tagged enum to `Vec<u8>` and back with postcard.
**When to use:** All `PeerMessage` serialization in `periphore-protocol` tests and in `periphore-net` codec.

```rust
// Source: Context7 /websites/rs_postcard_postcard (verified)
use serde::{Serialize, Deserialize};
use postcard::{to_allocvec, from_bytes};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerMessage {
    Hello { protocol_version: u32, fingerprint: [u8; 32] },
    Ping { timestamp: u64 },
    Pong { timestamp: u64 },
    MouseMove { dx: i32, dy: i32 },
    // ... all 15+ variants
    Bye,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_ping() {
        let msg = PeerMessage::Ping { timestamp: 12345 };
        let bytes: Vec<u8> = to_allocvec(&msg).unwrap();
        let decoded: PeerMessage = from_bytes(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn round_trip_all_variants() {
        // Test each variant; unit variants serialize to single discriminant byte
        let cases: Vec<PeerMessage> = vec![
            PeerMessage::Hello { protocol_version: 1, fingerprint: [0u8; 32] },
            PeerMessage::MouseMove { dx: -100, dy: 200 },
            PeerMessage::KeyEvent { scancode: 0x1E, pressed: true, modifiers: 0 },
            PeerMessage::Bye,
            // ... full coverage
        ];
        for msg in cases {
            let bytes = to_allocvec(&msg).unwrap();
            let decoded: PeerMessage = from_bytes(&bytes).unwrap();
            assert_eq!(msg, decoded);
        }
    }
}
```

**Note:** Use `postcard::to_allocvec` (requires `alloc` feature) for heap-allocated `Vec<u8>` on non-embedded targets. The `alloc` feature is the correct one for daemon code — not `heapless`.

### Pattern 5: LengthDelimitedCodec Framing

**What:** Wrap a `TcpStream` (or `UnixStream`) in length-delimited frame codec. Default is 4-byte big-endian length header.
**When to use:** `periphore-net` codec (Phase 6); stub-used in protocol crate for documentation.

```rust
// Source: Context7 /websites/rs_tokio-util (verified)
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use bytes::BytesMut;
use futures::{SinkExt, StreamExt};

fn framed_connection(stream: TcpStream) -> Framed<TcpStream, LengthDelimitedCodec> {
    // Default: 4-byte big-endian length header — correct for this project (D-13)
    Framed::new(stream, LengthDelimitedCodec::new())
}

// Custom builder if you need to confirm defaults explicitly:
fn framed_connection_explicit(stream: TcpStream) -> Framed<TcpStream, LengthDelimitedCodec> {
    LengthDelimitedCodec::builder()
        .length_field_type::<u32>()  // 4-byte header
        .big_endian()                // big-endian (default, but explicit)
        .new_framed(stream)
}
```

**Codec wrapping pattern** (for postcard encode/decode in the codec):
```rust
// The codec receives BytesMut frames and encodes/decodes PeerMessage
// This lives in periphore-net (Phase 6), not periphore-protocol
// Protocol crate defines types; net crate wraps in codec
```

### Pattern 6: Tokio Unix Domain Socket Server

**What:** IPC server that accepts connections, reads JSON-lines, dispatches to daemon.
**When to use:** `periphore-ipc` `server.rs`.

```rust
// Source: Context7 /websites/rs_tokio (verified)
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use std::path::Path;
use std::fs;

pub async fn serve(
    socket_path: &Path,
    cmd_tx: mpsc::Sender<IpcCommand>,
) -> std::io::Result<()> {
    // Remove stale socket from previous unclean shutdown
    let _ = fs::remove_file(socket_path);

    // Ensure parent directory exists
    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(socket_path)?;

    // Set permissions to 0600 (owner read/write only)
    // Note: UnixListener::bind respects umask; set explicitly:
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(socket_path, fs::Permissions::from_mode(0o600))?;
    }

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let tx = cmd_tx.clone();
                tokio::spawn(handle_connection(stream, tx));
            }
            Err(e) => {
                tracing::error!("IPC accept error: {e}");
            }
        }
    }
}

async fn handle_connection(stream: UnixStream, tx: mpsc::Sender<IpcCommand>) {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
        match serde_json::from_str::<IpcRequest>(line.trim()) {
            Ok(req) => { tx.send(IpcCommand::from(req)).await.ok(); }
            Err(e) => { tracing::warn!("Bad IPC request: {e}"); }
        }
        line.clear();
    }
}
```

**Cleanup on shutdown:**
```rust
// In main's shutdown handler:
let _ = std::fs::remove_file(&socket_path);
```

### Pattern 7: Signal Handling in Tokio Daemon

**What:** Handle SIGTERM and SIGHUP for clean daemon shutdown.
**When to use:** `periphored` `main.rs`.

```rust
// Source: Context7 /websites/rs_tokio (verified)
use tokio::signal::unix::{signal, SignalKind};

async fn run() -> anyhow::Result<()> {
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sighup  = signal(SignalKind::hangup())?;

    let mut tasks = tokio::task::JoinSet::new();
    // ... spawn ipc task, etc.

    tokio::select! {
        _ = sigterm.recv() => {
            tracing::info!("SIGTERM received — shutting down");
        }
        _ = sighup.recv() => {
            tracing::info!("SIGHUP received — reloading config");
            // config reload logic here
        }
        result = tasks.join_next() => {
            if let Some(Err(e)) = result {
                tracing::error!("Task panicked: {e}");
            }
        }
    }

    // Cleanup: remove socket
    let _ = std::fs::remove_file(&socket_path);
    Ok(())
}
```

### Pattern 8: Platform Socket Path via `directories`

**What:** Resolve the correct Unix domain socket path per platform without manual env var parsing.
**When to use:** `periphore-ipc` `path.rs`.

```rust
// Source: Context7 /git_codeberg_org/dirs_directories-rs (verified)
use directories::ProjectDirs;
use std::path::PathBuf;

pub fn socket_path() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("", "", "periphore") {
        if let Some(runtime) = dirs.runtime_dir() {
            // Linux: $XDG_RUNTIME_DIR/periphore/periphore.sock
            return runtime.join("periphore.sock");
        }
    }

    // macOS fallback: $TMPDIR/periphore/periphore.sock
    // (runtime_dir() returns None on macOS; TMPDIR is the standard)
    let tmp = std::env::var("TMPDIR")
        .unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(tmp).join("periphore").join("periphore.sock")
}
```

**Note:** `ProjectDirs::from("", "", "periphore")` on Linux maps `runtime_dir()` to `$XDG_RUNTIME_DIR/periphore`. On macOS, `runtime_dir()` returns `None` (no XDG standard on macOS); use `$TMPDIR` fallback as specified in D-17. [VERIFIED: Context7 directories docs]

### Pattern 9: clap v4 Derive for Daemon Binary

**What:** Minimal clap v4 derive setup for `periphored --help`.
**When to use:** `periphored/src/main.rs`.

```rust
// Source: Context7 /websites/rs_clap (verified)
use clap::Parser;

/// Periphore input sharing daemon
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<std::path::PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    // ... load config, start IPC, handle signals
    Ok(())
}
```

### Anti-Patterns to Avoid

- **Bare path refs inside crate Cargo.tomls:** Never write `periphore-protocol = { path = "../periphore-protocol" }` inside a crate. Always `{ workspace = true }`. Bare paths bypass workspace dep management and break feature activation. [CITED: astral-sh/uv pattern]
- **Config struct with `Serialize` derive:** Adding `#[derive(Serialize)]` to the main `Config` struct creates a code path to write it. CFG-01 requires no auto-write. Enforce at compile time: no `Serialize` impl, no write path. [CITED: D-23]
- **`Figment::new().merge(cli).merge(env).merge(toml).merge(defaults)`:** This is the wrong order — CLI would be lowest priority. Each `.merge()` adds a higher-priority source; the last call wins. The correct order always ends with the highest-priority source (CLI). [VERIFIED: Context7 figment docs]
- **`postcard::to_vec` (heapless) for daemon code:** Use `postcard::to_allocvec` (the `alloc` feature) for standard heap allocation. `to_vec` requires the `heapless` feature and a compile-time buffer size — inappropriate for a daemon. [VERIFIED: Context7 postcard docs]
- **`LengthDelimitedCodec` without checking default header size:** The default is u32 big-endian (4 bytes), which is correct. Document this explicitly. If using the builder, call `.length_field_type::<u32>().big_endian()` to make the intent clear. [VERIFIED: Context7 tokio-util docs]
- **Missing socket cleanup before bind:** If the daemon crashed previously, the stale socket file blocks `UnixListener::bind`. Always `fs::remove_file(path).ok()` before binding. [ASSUMED — standard Unix pattern]
- **`workspace.lints` retrofitted later:** Setting up lints after 11 crates are written is painful — every crate needs `[lints] workspace = true` added. Do it in task 1. [CITED: WORKSPACE-PATTERNS.md]
- **Using `anyhow` in library crates:** `thiserror` for typed errors in library crates; `anyhow` only in binary entry points (`periphored`, `periphore`). Library errors must be typed for callers to handle. [ASSUMED — Rust ecosystem consensus]

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Platform config/runtime dirs | `$XDG_RUNTIME_DIR` env var reading | `directories` crate (`ProjectDirs`) | XDG spec has complex fallback chains; macOS `$TMPDIR` vs `$HOME/Library` is non-obvious |
| Config layering with precedence | Manual TOML parse + env var override | `figment` | Precedence logic, type coercion, error provenance are all non-trivial |
| Binary framing over streams | Custom 4-byte header read loop | `tokio-util` `LengthDelimitedCodec` | Handles partial reads, backpressure, frame splitting correctly |
| JSON-lines serialization | `format!("{}\n", serde_json::to_string(...))` | `serde_json` with explicit newline | Actually fine to write manually, but use serde_json for deserialization correctness |
| CLI arg parsing | Manual `std::env::args()` parsing | `clap` v4 derive | --help, --version, error formatting, env var integration all handled automatically |
| Workspace lint policy | Per-crate `#![deny(...)]` attributes | `[workspace.lints]` | Workspace lints are synchronized; per-crate attributes diverge and are harder to update |

**Key insight:** In a foundational phase, the real danger is hand-rolling infrastructure (config layering, path resolution, framing) that has subtle correctness requirements. Every item in this table represents a class of bugs (partial read, wrong fallback path, config key casing) that existing crates have already solved.

---

## Common Pitfalls

### Pitfall 1: Figment Merge Order (Inverted Priority)

**What goes wrong:** Developer writes `Figment::new().merge(cli).merge(toml).merge(env)` thinking "CLI overrides env overrides TOML." Actually: each `.merge()` incoming wins — TOML wins over CLI. CLI args are silently ignored.

**Why it happens:** The method name `.merge()` suggests combining, not "this wins." The last `.merge()` call's source has highest priority — counterintuitive.

**How to avoid:** Always write the chain lowest-to-highest: `Figment::from(Serialized::defaults(...)).merge(Toml::file(...)).merge(Env::prefixed(...)).merge(cli_overrides)`. The last thing merged wins.

**Warning signs:** Config file values appear even when CLI overrides are passed; env vars don't override file values.

[VERIFIED: Context7 figment docs — "When keys conflict, values from the incoming provider take precedence"]

### Pitfall 2: Stale Unix Socket File on Restart

**What goes wrong:** Daemon crashes or is killed without cleanup. On restart, `UnixListener::bind(path)` fails with `Address already in use`. Daemon fails to start.

**Why it happens:** Unix domain sockets leave a filesystem artifact. Unlike TCP ports, the OS does not automatically clean up UDS files after process death.

**How to avoid:** Always call `std::fs::remove_file(&socket_path).ok()` before `UnixListener::bind`. The `.ok()` suppresses the error if the file doesn't exist.

**Warning signs:** Daemon fails to start after unclean shutdown; "address already in use" in logs.

[ASSUMED — standard Unix programming pattern; widely documented]

### Pitfall 3: IPC Socket Permissions (Mode 0600)

**What goes wrong:** `UnixListener::bind` creates the socket file with permissions derived from the process umask. If umask is permissive, other users can connect to the daemon's IPC socket.

**Why it happens:** `bind()` does not set explicit permissions; it honors umask. Default umask is typically `0022`, leaving mode `0644` — world-readable.

**How to avoid:** After binding, explicitly call `fs::set_permissions(path, Permissions::from_mode(0o600))` using `std::os::unix::fs::PermissionsExt`. Do this immediately after bind, before accepting connections.

**Warning signs:** `ls -la /run/user/*/periphore/` shows `-rw-r--r--` instead of `-rw-------`.

[ASSUMED — Unix socket security requirement from D-17]

### Pitfall 4: `postcard` Feature Selection

**What goes wrong:** Protocol crate compiles with the `heapless` feature instead of `alloc`. `to_vec::<32>(&msg)` requires knowing the max serialized size at compile time — daemon code will fail with buffer overflows for large payloads (e.g., `TopologyAdvertise` with many monitors).

**Why it happens:** postcard docs prominently feature `heapless` for embedded use cases. Daemon code should use `to_allocvec` from the `alloc` feature.

**How to avoid:** In `[workspace.dependencies]`: `postcard = { version = "1.1", features = ["alloc"] }`. Use `postcard::to_allocvec(&msg)` everywhere.

**Warning signs:** Compilation errors on `to_allocvec` if wrong feature selected; panics at runtime for large payloads with `heapless`.

[VERIFIED: Context7 postcard docs — `to_allocvec` requires `alloc` feature]

### Pitfall 5: Edition 2024 `extern crate` Changes

**What goes wrong:** Rust 2024 edition removes the implicit `extern crate` for macros from certain crates. Code like `#[macro_use] extern crate serde` stops working.

**Why it happens:** Edition 2024 (stabilized in Rust 1.85) makes 2018+ edition idioms mandatory — no `extern crate`, no `extern crate serde_derive`.

**How to avoid:** Use `use serde::{Serialize, Deserialize}` throughout. Never write `extern crate`. This is already standard practice since Rust 2018, so it's unlikely to bite — but any copy-pasted pre-2018 examples from Stack Overflow will fail.

**Warning signs:** Compiler error "can't find crate for X" on legitimate crates.

[ASSUMED — Edition 2024 behavior based on Rust edition history; Rust 1.95 confirms edition 2024 is available]

### Pitfall 6: Workspace Binary Not in `default-members`

**What goes wrong:** `cargo build` at workspace root builds nothing (or only one binary if `default-members` is misconfigured). CI pipeline "succeeds" while building no actual code.

**Why it happens:** When `members = ["crates/*"]` is set without `default-members`, `cargo build` builds all members — including library stubs that pass trivially. Or it builds only the library but not the binaries.

**How to avoid:** Set `default-members = ["crates/periphored", "crates/periphore"]` explicitly. Verify by running `cargo build` (not `cargo build --workspace`) and checking that both `target/debug/periphored` and `target/debug/periphore` are created.

**Warning signs:** `cargo build` succeeds but `./target/debug/periphored` does not exist.

[CITED: WORKSPACE-PATTERNS.md — uv/typst verified pattern]

### Pitfall 7: Config Struct Acquires `Serialize` Over Time

**What goes wrong:** A developer adds `#[derive(Serialize, Deserialize)]` to `Config` "for convenience" (e.g., to log the config as JSON). This creates a path where `serde_json::to_string(&config)` exists, making it trivial to accidentally write config back to disk in a later phase.

**Why it happens:** Rust derive macros are additive and there's no compile-time check for "this type must never be serialized to a file."

**How to avoid:** Explicitly document in the `periphore-config` crate: `Config` derives only `Deserialize`, never `Serialize`. Use `#[derive(Debug)]` for logging needs. CFG-01 is a hard invariant.

**Warning signs:** `[dependencies]` in `periphore-config` includes anything file-write-related; PR adds `Serialize` to `Config`.

[CITED: D-23, CFG-01]

---

## Code Examples

### Full PeerMessage Enum (all 15+ variants)
```rust
// periphore-protocol/src/peer.rs
use serde::{Serialize, Deserialize};
use crate::types::{MonitorInfo, Edge, EdgeMapping, InputEvent, KeyEventData, MouseEventData};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PeerMessage {
    // Handshake
    Hello      { protocol_version: u32, fingerprint: [u8; 32], public_key: Vec<u8> },
    HelloAck   { fingerprint: [u8; 32], public_key: Vec<u8>, accepted: bool },

    // Topology
    TopologyAdvertise { monitors: Vec<MonitorInfo> },
    TopologyPropose   { edges: Vec<EdgeMapping> },
    TopologyAccept,
    TopologyReject    { reason: String },

    // Focus token
    FocusTransfer { entry_edge: Edge, entry_position: f64, sequence: u64 },
    FocusAck      { sequence: u64 },
    FocusReclaim,

    // Input events
    MouseMove   { dx: i32, dy: i32 },
    MouseButton { button: u8, pressed: bool },
    MouseScroll { dx: i32, dy: i32 },
    KeyEvent    { scancode: u32, pressed: bool, modifiers: u8 },

    // Control
    Ping { timestamp: u64 },
    Pong { timestamp: u64 },
    Bye,
}
```

### Full IpcRequest Enum
```rust
// periphore-protocol/src/ipc.rs
use serde::{Serialize, Deserialize};
use crate::types::{InputEvent, Edge};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum IpcRequest {
    GetStatus,
    ListPeers,
    GetTopology,
    AcceptFingerprint { fingerprint: String },
    RejectFingerprint { fingerprint: String },
    ReloadConfig,
    InjectInputEvent  { event: InputEvent },
    SimulateEdgeCross { edge: Edge, position: f64 },
    GetState,
    GetPendingVerifications,
    GetIdenticon      { fingerprint: String },
    GetWordPhrase     { fingerprint: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum IpcResponse {
    Status { running: bool, fingerprint: Option<String> },
    Peers  { peers: Vec<String> },
    Ok,
    Error  { message: String },
    // ... extend in Phase 4
}
```

### Supporting Types
```rust
// periphore-protocol/src/types.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub id:     u32,
    pub width:  u32,
    pub height: u32,
    pub x:      i32,
    pub y:      i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Edge { Left, Right, Top, Bottom }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeMapping {
    pub from_monitor: u32,
    pub from_edge:    Edge,
    pub to_peer:      String,   // peer fingerprint
    pub to_monitor:   u32,
    pub to_edge:      Edge,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputEvent {
    Mouse(MouseEventData),
    Key(KeyEventData),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MouseEventData { pub dx: i32, pub dy: i32 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyEventData   { pub scancode: u32, pub pressed: bool, pub modifiers: u8 }
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `extern crate serde` + `#[macro_use]` | `use serde::{Serialize, Deserialize}` | Rust 2018 | Edition 2024 enforces this |
| Per-crate lint attributes (`#![deny(warnings)]`) | `[workspace.lints]` + `[lints] workspace = true` | Rust 1.73 (stabilized) | Synchronized lint policy across workspace |
| `resolver = "1"` (feature unification) | `resolver = "2"` (independent features per crate) | Rust 1.51 | Required for correct feature resolution in workspaces |
| Manual config parsing (toml-rs + dotenv) | `figment` layered providers | ~2020 | Declarative merge chain with error provenance |
| `bincode` for wire protocols | `postcard` | ~2021 | More compact, no_std-compatible, deterministic |
| Actor frameworks (Actix) for concurrency | `tokio::mpsc` bounded channels | ongoing | Less overhead, no supervisor complexity for this use case |

**Deprecated/outdated:**
- `bincode` v1 API: The v2 API changed significantly; if examples reference `bincode::serialize`, they use v1. The project uses `postcard` instead, so this is irrelevant.
- `clap` v2/v3 builder API: All clap usage should use the v4 derive API. Old builder-pattern examples are still common in search results.
- `structopt`: Merged into clap v3+; do not use as a separate crate.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `fs::remove_file().ok()` before bind is sufficient to clear a stale socket | Pitfall 2: Stale Socket | Low — this is universally documented Unix practice; risk is negligible |
| A2 | `fs::set_permissions(..., 0o600)` after bind sets socket permissions correctly on macOS and Linux | Pitfall 3: Socket Permissions | Low — `PermissionsExt` is `#[cfg(unix)]` and works on both targets |
| A3 | `ProjectDirs::from("", "", "periphore").runtime_dir()` returns `None` on macOS, requiring the `$TMPDIR` fallback | Pattern 8: Socket Path | Medium — if `directories` 6.0 changed macOS behavior, socket path would be wrong; verify in Wave 0 test |
| A4 | `anyhow` in binary entry points, `thiserror` in library crates | Anti-Patterns | Low — this is ecosystem consensus but not enforced; wrong choice doesn't cause bugs, only poor DX |
| A5 | Edition 2024 is stable in Rust 1.95.0 | Pattern 1: Workspace Cargo.toml | Verified via `rustc --edition 2024 --help` output showing "stable edition is 2024" |
| A6 | `sha2 = "0.10.9"` is the correct stable version (0.11.0 is an RC) | Standard Stack | Medium — if 0.11.0 stabilized, using 0.10.9 misses API improvements; low risk for Phase 1 (identity is a stub) |

**Verified claims are tagged [VERIFIED] or [CITED] inline. Only A1–A6 remain as assumptions.**

---

## Open Questions (RESOLVED)

1. **`directories` crate behavior on macOS for `runtime_dir()`**
   - What we know: `ProjectDirs::from` documentation shows `runtime_dir()` returns `None` on macOS (no XDG on macOS)
   - What's unclear: Whether `directories` 6.0 introduced a macOS-specific runtime path (e.g., `$HOME/Library/Caches`)
   - **RESOLVED:** `runtime_dir()` returns `None` on macOS; the `$TMPDIR/periphore/periphore.sock` fallback specified in D-17 handles macOS correctly. The `socket_path_ends_in_periphore_sock` integration test in Plan 04 verifies this at runtime. No behavior change from `directories` 6.0 needed — the fallback path is the correct macOS answer.

2. **`sha2` 0.11.0 stability status**
   - What we know: crates.io shows `0.11.0` as latest, but cargo search labels it distinctly; crates.io API non-rc query returns `0.10.9` as latest non-rc
   - What's unclear: Whether `0.11.0` final stable was published after our verification
   - **RESOLVED:** Using `sha2 = "0.10.9"` (last confirmed stable) in workspace deps for Phase 1. Will re-evaluate at Phase 2 when identity is implemented. `periphore-identity` is a stub in Phase 1 — sha2 version is not observable until Phase 2.

3. **Figment `clap` feature gate pattern**
   - What we know: `periphore-config` needs a `clap` feature that enables CLI override struct
   - What's unclear: Whether the CLI override struct should be in `periphore-config` or entirely in `periphore-cli`
   - **RESOLVED:** CLI override struct goes in `periphore-config` behind a `clap` feature gate per D-25 (locked decision from CONTEXT.md). `periphore-cli` activates the feature via `periphore-config = { workspace = true, features = ["clap"] }`. This is already implemented in Plan 01 Cargo.toml stubs. No further clarification needed.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (`rustc`, `cargo`) | All compilation | Yes | 1.95.0 (stable) | — |
| Edition 2024 support | Workspace Cargo.toml | Yes | Confirmed in Rust 1.85+ | — |
| `cargo clippy` | Workspace lint enforcement | Yes | Ships with rustc 1.95.0 | — |
| `cargo test` | Protocol round-trip tests | Yes | Ships with cargo 1.95.0 | — |
| Unix domain sockets | `periphore-ipc` | Yes | macOS Darwin 25.4.0 supports UDS | — |
| `$XDG_RUNTIME_DIR` env var | Linux socket path | N/A (macOS dev machine) | — | `$TMPDIR` fallback (D-17) |
| `$TMPDIR` env var | macOS socket path | Yes | Set by macOS launchd | `/tmp` hardcoded fallback |
| Pre-commit hooks (`prek`) | `commitizen`, `check-merge-conflict`, etc. | Yes | prek.toml uses v6.0.0 hooks | — |

**Missing dependencies with no fallback:** None — all required tools are present.

**Note:** Development is on macOS (`darwin 25.4.0`). Linux-specific behavior (`$XDG_RUNTIME_DIR`) will be tested in CI, not on the developer machine.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (`#[test]`, `#[cfg(test)]`) |
| Config file | None required — Cargo handles test discovery |
| Quick run command | `cargo test -p periphore-protocol` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CFG-01 | Config struct has no `Serialize` impl (compile-time) | Compile check | `cargo build -p periphore-config` | No — Wave 0 |
| CFG-01 | No write paths in `periphore-config` source | Negative compile test | Build succeeds without any `std::fs::write` in config crate | No — Wave 0 |
| IPC-01 | Daemon creates socket at platform path on startup | Integration | `cargo test -p periphore-ipc -- ipc::tests::socket_creates` | No — Wave 0 |
| IPC-01 | Socket removed on clean shutdown | Integration | `cargo test -p periphore-ipc -- ipc::tests::socket_removed_on_shutdown` | No — Wave 0 |
| IPC-02 | `GetStatus` request returns response over socket | Integration | `cargo test -p periphore-ipc -- ipc::tests::get_status_response` | No — Wave 0 |
| IPC-02 | `InjectInputEvent` accepted without network peer | Integration | `cargo test -p periphore-ipc -- ipc::tests::inject_input_no_peer` | No — Wave 0 |
| Protocol | `PeerMessage` all variants round-trip via postcard | Unit | `cargo test -p periphore-protocol -- peer::tests` | No — Wave 0 |
| Protocol | `IpcRequest` all variants round-trip via serde_json | Unit | `cargo test -p periphore-protocol -- ipc::tests` | No — Wave 0 |
| Build | `cargo build --workspace` succeeds | Build check | `cargo build --workspace` | No — Wave 0 |
| Build | `periphore --help` produces output | Smoke | `./target/debug/periphore --help` | No — Wave 0 |
| Build | `periphored --help` produces output | Smoke | `./target/debug/periphored --help` | No — Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p periphore-protocol && cargo clippy --workspace`
- **Per wave merge:** `cargo test --workspace && cargo build --workspace`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/periphore-protocol/src/peer.rs` — covers `PeerMessage` round-trip tests
- [ ] `crates/periphore-protocol/src/ipc.rs` — covers `IpcRequest`/`IpcResponse` round-trip tests
- [ ] `crates/periphore-ipc/tests/socket.rs` — covers IPC socket lifecycle integration tests
- [ ] `crates/periphore-config/tests/config.rs` — covers config layering and no-write invariant
- [ ] Root `Cargo.toml` with workspace configuration — entire workspace needs creation

---

## Security Domain

> `security_enforcement` is enabled. `security_asvs_level = 1`.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No — no user login in this phase | — |
| V3 Session Management | No — IPC is local-only, no sessions | — |
| V4 Access Control | Yes — IPC socket should only be accessible by the daemon owner | Unix socket permissions `0600` (D-17) |
| V5 Input Validation | Yes — JSON-lines parsing from IPC clients | `serde_json` with typed enum; unknown variants return `IpcResponse::Error` |
| V6 Cryptography | No — Phase 1 stubs identity; no crypto operations | — |

### Known Threat Patterns for This Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Local process hijacks IPC socket (pre-bind) | Elevation of Privilege | `0600` permissions + bind immediately at startup before listening |
| Malformed JSON-lines causing daemon panic | Denial of Service | `serde_json::from_str` returns `Result`; never `.unwrap()` on IPC input; log and skip |
| Config file with malicious paths/values | Tampering | Figment deserializes to typed struct; no `eval`-style config; struct types reject invalid values at parse |
| Stale socket file from previous instance | Denial of Service | Remove stale socket before bind; document in startup sequence |

**Phase 1 security posture:** Conservative. No network exposure. IPC is local-only with owner-only permissions. Crypto is stubbed — implemented in Phase 2. The primary security requirement (CFG-01, no config auto-write) is enforced at compile time via type system.

---

## Project Constraints (from CLAUDE.md)

The following directives from `CLAUDE.md` apply to this phase and must be honored by the planner:

| Directive | Impact on Phase 1 |
|-----------|------------------|
| Language: Rust, runtime: Tokio | All implementation is `async`/`await`; `#[tokio::main]` on daemon entry |
| Protocol framing: `tokio-util LengthDelimitedCodec` + `postcard`/`serde` | Protocol crate uses postcard; framing scaffold in protocol crate (net implements in Phase 6) |
| Config: `clap` v4 + `figment` + TOML | Use derive API; use Figment merge chain; no config writes |
| Logging: `tracing` + `tracing-subscriber` | Initialize `tracing_subscriber` in `periphored` main; library crates use `tracing::` macros only |
| Config discipline: NEVER auto-writes | No `Serialize` on Config; no `fs::write` in config crate; enforced in tests |
| No GUI in v1 | No UI code, no window creation, no display dependencies in Phase 1 |
| Platforms: macOS and Linux only | `#[cfg(unix)]` guards on UDS code; no Windows imports |
| Conventional commits via commitizen | All commits in this phase follow `feat:`, `chore:`, `test:`, `build:` prefixes |
| `TCP_NODELAY` must be set immediately | Noted for Phase 6 net; does not apply to Phase 1 |
| Binary crate terminology: `periphore` (CLI), `periphored` (daemon) | Crate names confirmed; binary names confirmed |
| Build order: protocol → config+identity → core+ipc+cli → net → capture+inject | Task ordering in Phase 1 follows this; protocol types defined before IPC uses them |

---

## Sources

### Primary (HIGH confidence)
- Context7 `/sergiobenitez/figment` — merge/join semantics, Serialized::defaults, Env::prefixed, Toml::file patterns
- Context7 `/websites/rs_tokio` — `UnixListener::accept`, `SignalKind::terminate()`, `SignalKind::hangup()` patterns
- Context7 `/websites/rs_tokio-util` — `LengthDelimitedCodec::new()`, `Framed`, builder API
- Context7 `/websites/rs_postcard_postcard` — `to_allocvec`, `from_bytes`, alloc feature
- Context7 `/websites/rs_clap` — `#[derive(Parser)]`, `#[derive(Subcommand)]`, `#[command(version, about)]`
- Context7 `/git_codeberg_org/dirs_directories-rs` — `ProjectDirs::from`, `runtime_dir()`, platform path behavior
- `cargo search` registry — tokio 1.52.1, tokio-util 0.7.18, serde 1.0.228, postcard 1.1.3, figment 0.10.19, clap 4.6.1, tracing 0.1.44, tracing-subscriber 0.3.23, directories 6.0.0, thiserror 2.0.18, bytes 1.11.1, serde_json 1.0.149 [VERIFIED: cargo registry]
- crates.io API — ed25519-dalek 2.2.0 (last stable non-pre), sha2 0.10.9 (last stable non-rc) [VERIFIED: crates.io API]
- `.planning/research/ARCHITECTURE.md` — crate structure, IPC design, channel topology
- `.planning/research/WORKSPACE-PATTERNS.md` — uv/typst workspace patterns (verified against production projects)
- `.planning/research/STACK.md` — library selections with rationale
- `.planning/research/PITFALLS.md` — implementation landmines
- `rustc --edition 2024 --help` — confirms edition 2024 stable in Rust 1.95.0 [VERIFIED: toolchain check]

### Secondary (MEDIUM confidence)
- `.planning/phases/01-workspace-protocol-foundation/01-CONTEXT.md` — all locked decisions D-01 through D-31
- `.planning/phases/01-workspace-protocol-foundation/01-DISCUSSION-LOG.md` — rationale behind decisions

### Tertiary (LOW confidence)
- None — all claims in this research are verified or cited

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all versions verified via cargo registry and crates.io API
- Architecture: HIGH — derived from locked CONTEXT.md decisions + verified reference projects (uv, typst)
- Figment layering: HIGH — verified against Context7 official docs with code examples
- Pitfalls: HIGH for workspace/Figment/socket pitfalls (documented patterns); MEDIUM for socket permissions (standard practice, not tested)
- Code examples: HIGH — all from Context7 verified sources

**Research date:** 2026-04-22
**Valid until:** 2026-05-22 (stable crates; check for ed25519-dalek 3.x stable release before Phase 2)
