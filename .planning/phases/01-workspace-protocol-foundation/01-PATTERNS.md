# Phase 1: Workspace & Protocol Foundation — Pattern Map

**Mapped:** 2026-04-22
**Files analyzed:** 38 (all new — greenfield project)
**Analogs found:** 0 / 38 (no existing Rust source; all patterns sourced from research documents)

---

## Greenfield Status

This is a greenfield Rust project. There are zero existing `.rs` source files in the repository. The only artifacts in the codebase are:

- `/prek.toml` — pre-commit hook configuration (commitizen, pre-commit-hooks v6.0.0)
- `/cz.toml` — commitizen configuration (conventional commits, semver, `version_provider = "cargo"`)
- `/.gitignore` — standard Rust + macOS + Linux + Node ignore patterns

The planner MUST create all files from scratch. All pattern authority comes from the research documents listed under "Pattern Sources" below.

---

## File Classification

| New File | Role | Data Flow | Pattern Source | Match Quality |
|----------|------|-----------|----------------|---------------|
| `Cargo.toml` (workspace root) | config | transform | `01-RESEARCH.md` Pattern 1 + `WORKSPACE-PATTERNS.md` §1 | research-exact |
| `crates/periphore-protocol/Cargo.toml` | config | — | `01-RESEARCH.md` Pattern 2 + `WORKSPACE-PATTERNS.md` §6 | research-exact |
| `crates/periphore-protocol/src/lib.rs` | utility | transform | `01-RESEARCH.md` §Code Examples | research-exact |
| `crates/periphore-protocol/src/peer.rs` | model | request-response | `01-RESEARCH.md` Pattern 4 + §Code Examples | research-exact |
| `crates/periphore-protocol/src/ipc.rs` | model | request-response | `01-RESEARCH.md` §Code Examples | research-exact |
| `crates/periphore-protocol/src/types.rs` | model | transform | `01-RESEARCH.md` §Code Examples | research-exact |
| `crates/periphore-config/Cargo.toml` | config | — | `01-RESEARCH.md` Pattern 2 | research-exact |
| `crates/periphore-config/src/lib.rs` | service | CRUD | `01-RESEARCH.md` Pattern 3 | research-exact |
| `crates/periphore-config/src/schema.rs` | model | transform | `01-RESEARCH.md` Pattern 3 + §Phase Requirements CFG-01 | research-exact |
| `crates/periphore-ipc/Cargo.toml` | config | — | `01-RESEARCH.md` Pattern 2 | research-exact |
| `crates/periphore-ipc/src/lib.rs` | service | event-driven | `01-RESEARCH.md` Pattern 6 | research-exact |
| `crates/periphore-ipc/src/server.rs` | service | event-driven | `01-RESEARCH.md` Pattern 6 | research-exact |
| `crates/periphore-ipc/src/path.rs` | utility | transform | `01-RESEARCH.md` Pattern 8 | research-exact |
| `crates/periphore-ipc/tests/socket.rs` | test | event-driven | `01-RESEARCH.md` §Validation Architecture | research-partial |
| `crates/periphore-identity/Cargo.toml` | config | — | `01-RESEARCH.md` Pattern 2 | research-exact |
| `crates/periphore-identity/src/lib.rs` | model | — | `WORKSPACE-PATTERNS.md` §5 (stub only) | research-exact |
| `crates/periphore-core/Cargo.toml` | config | — | `01-RESEARCH.md` Pattern 2 | research-exact |
| `crates/periphore-core/src/lib.rs` | model | — | `WORKSPACE-PATTERNS.md` §5 (stub only) | research-exact |
| `crates/periphore-net/Cargo.toml` | config | — | `01-RESEARCH.md` Pattern 2 | research-exact |
| `crates/periphore-net/src/lib.rs` | model | — | `WORKSPACE-PATTERNS.md` §5 (stub only) | research-exact |
| `crates/periphore-capture/Cargo.toml` | config | — | `01-RESEARCH.md` Pattern 2 | research-exact |
| `crates/periphore-capture/src/lib.rs` | model | — | `WORKSPACE-PATTERNS.md` §5 (stub only) | research-exact |
| `crates/periphore-inject/Cargo.toml` | config | — | `01-RESEARCH.md` Pattern 2 | research-exact |
| `crates/periphore-inject/src/lib.rs` | model | — | `WORKSPACE-PATTERNS.md` §5 (stub only) | research-exact |
| `crates/periphore-cli/Cargo.toml` | config | — | `01-RESEARCH.md` Pattern 2 | research-exact |
| `crates/periphore-cli/src/lib.rs` | service | request-response | `WORKSPACE-PATTERNS.md` §4 (stub only) | research-exact |
| `crates/periphore/Cargo.toml` | config | — | `01-RESEARCH.md` Pattern 2 | research-exact |
| `crates/periphore/src/main.rs` | utility | request-response | `01-RESEARCH.md` Pattern 9 (stub) | research-exact |
| `crates/periphored/Cargo.toml` | config | — | `01-RESEARCH.md` Pattern 2 | research-exact |
| `crates/periphored/src/main.rs` | service | event-driven | `01-RESEARCH.md` Patterns 7+9 + `ARCHITECTURE.md` §4 | research-exact |
| `crates/periphore-config/tests/config.rs` | test | CRUD | `01-RESEARCH.md` §Validation Architecture | research-partial |

---

## Pattern Assignments

### `Cargo.toml` (workspace root) — config

**Pattern source:** `01-RESEARCH.md` Pattern 1 (lines 263–316) + `WORKSPACE-PATTERNS.md` §1, §2, §3

**Workspace scaffold pattern:**
```toml
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

# External dependencies — pinned per RESEARCH.md §Standard Stack
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

**Critical rules:**
- `periphore` and `periphored` binary crates are NOT listed in `[workspace.dependencies]` — they are binaries, not libraries consumed by other crates
- All 9 library crates ARE listed with both `path` and `version`
- `resolver = "2"` is mandatory for correct feature resolution in workspaces (D-01)
- `edition = "2024"` requires Rust 1.85+ (confirmed available: Rust 1.95.0)

---

### Individual Crate `Cargo.toml` — general pattern (applies to all 11 crates)

**Pattern source:** `01-RESEARCH.md` Pattern 2 (lines 320–346) + `WORKSPACE-PATTERNS.md` §6

**Standard crate Cargo.toml pattern:**
```toml
[package]
name = "periphore-CRATE-NAME"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
# All deps as { workspace = true } — never bare path or version refs
some-dep = { workspace = true }
```

**Thin/foundational crates** (`periphore-protocol`, `periphore-identity`) additionally include:
```toml
[lib]
doctest = false
test = false
```

**Feature-gated crate example** (`periphore-config` with optional `clap` feature per D-25):
```toml
[features]
clap = ["dep:clap"]

[dependencies]
figment     = { workspace = true }
serde       = { workspace = true }
thiserror   = { workspace = true }
clap        = { workspace = true, optional = true }
```

**Consumer activating a feature** (in `periphore-cli/Cargo.toml`):
```toml
[dependencies]
periphore-config = { workspace = true, features = ["clap"] }
```

---

### `crates/periphore-protocol/src/peer.rs` — model, request-response

**Pattern source:** `01-RESEARCH.md` Pattern 4 (lines 392–439) + §Code Examples (lines 757–791)

**PeerMessage enum pattern:**
```rust
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

**Round-trip test pattern** (inline in `src/peer.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_all_variants() {
        let cases: Vec<PeerMessage> = vec![
            PeerMessage::Hello { protocol_version: 1, fingerprint: [0u8; 32], public_key: vec![] },
            PeerMessage::MouseMove { dx: -100, dy: 200 },
            PeerMessage::KeyEvent { scancode: 0x1E, pressed: true, modifiers: 0 },
            PeerMessage::Ping { timestamp: 12345 },
            PeerMessage::Bye,
            // all 15+ variants
        ];
        for msg in cases {
            let bytes = postcard::to_allocvec(&msg).unwrap();
            let decoded: PeerMessage = postcard::from_bytes(&bytes).unwrap();
            assert_eq!(msg, decoded);
        }
    }
}
```

**Critical:** Use `postcard::to_allocvec` (requires `features = ["alloc"]`), NOT `to_vec` (heapless). `to_vec` requires a compile-time buffer size — inappropriate for daemon use. (RESEARCH.md Pitfall 4)

---

### `crates/periphore-protocol/src/ipc.rs` — model, request-response

**Pattern source:** `01-RESEARCH.md` §Code Examples (lines 795–825)

**IpcRequest/IpcResponse pattern:**
```rust
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
}
```

**Note:** `IpcRequest` and `IpcResponse` both derive `Serialize` (unlike `Config` which must NOT derive `Serialize`). The IPC layer serializes responses to write back to clients — this is correct.

**Round-trip test pattern** for JSON-lines:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_get_status() {
        let req = IpcRequest::GetStatus;
        let json = serde_json::to_string(&req).unwrap();
        let decoded: IpcRequest = serde_json::from_str(&json).unwrap();
        // GetStatus has no fields — just verify it round-trips
        assert!(matches!(decoded, IpcRequest::GetStatus));
    }

    #[test]
    fn round_trip_inject_input_event() {
        use crate::types::{InputEvent, MouseEventData};
        let req = IpcRequest::InjectInputEvent {
            event: InputEvent::Mouse(MouseEventData { dx: 10, dy: -5 }),
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: IpcRequest = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, IpcRequest::InjectInputEvent { .. }));
    }
}
```

---

### `crates/periphore-protocol/src/types.rs` — model, transform

**Pattern source:** `01-RESEARCH.md` §Code Examples (lines 828–864)

**Supporting types pattern:**
```rust
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
pub struct KeyEventData { pub scancode: u32, pub pressed: bool, pub modifiers: u8 }
```

---

### `crates/periphore-protocol/src/lib.rs` — utility, re-export facade

**Pattern source:** `WORKSPACE-PATTERNS.md` §5 + `ARCHITECTURE.md` §3

**Re-export facade pattern:**
```rust
// periphore-protocol/src/lib.rs
// Re-export all public types from submodules for a flat public API.
// Consumers: use periphore_protocol::{PeerMessage, IpcRequest, Edge, ...};

pub mod peer;
pub mod ipc;
pub mod types;

// Re-export the most commonly used types at crate root
pub use peer::PeerMessage;
pub use ipc::{IpcRequest, IpcResponse};
pub use types::{Edge, EdgeMapping, InputEvent, KeyEventData, MonitorInfo, MouseEventData};
```

**Note:** Submodule approach chosen (as recommended by CONTEXT.md Claude's Discretion). Public re-exports at crate root give consumers a clean import path without requiring knowledge of the internal module structure.

---

### `crates/periphore-config/src/lib.rs` + `src/schema.rs` — service, CRUD

**Pattern source:** `01-RESEARCH.md` Pattern 3 (lines 348–387)

**Config load function pattern** (`src/lib.rs`):
```rust
use figment::{Figment, providers::{Format, Toml, Env, Serialized}};

pub use crate::schema::Config;

pub fn load(config_path: Option<&std::path::Path>) -> Result<Config, figment::Error> {
    let mut figment = Figment::from(Serialized::defaults(Config::default()));

    if let Some(path) = config_path {
        figment = figment.merge(Toml::file(path));
    }

    figment = figment.merge(Env::prefixed("PERIPHORE_").split("_"));
    // CLI args merged last by the binary entry point, not here (D-22)

    figment.extract()
}
```

**Config schema pattern** (`src/schema.rs`):
```rust
use serde::Deserialize;

// CRITICAL: Config intentionally does NOT derive Serialize.
// This enforces CFG-01 at compile time: no code path can serialize Config to disk.
// Use #[derive(Debug)] for logging needs only.
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub daemon:   DaemonConfig,
    pub logging:  LoggingConfig,
    pub peers:    Vec<PeerConfig>,
    pub topology: TopologyConfig,
    pub capture:  CaptureConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct DaemonConfig {
    pub socket_path: Option<std::path::PathBuf>,
    // ... daemon-specific fields
}

#[derive(Debug, Deserialize, Default)]
pub struct LoggingConfig {
    pub level: String,
    // ...
}

#[derive(Debug, Deserialize, Default)]
pub struct PeerConfig {
    pub fingerprint: String,
    pub host:        Option<String>,
    pub port:        Option<u16>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TopologyConfig {
    // edge mappings, alignment strategy
}

#[derive(Debug, Deserialize, Default)]
pub struct CaptureConfig {
    // captive vs seamless, capture device path
}
```

**Critical invariant:** `Config` MUST NEVER derive `Serialize`. No `use serde::Serialize` in this file. CFG-01 is enforced at the type system level. Any PR adding `Serialize` to `Config` violates the architectural constraint.

**Merge order is critical** (Pitfall 1 in RESEARCH.md): The last `.merge()` wins. Order must be: `Serialized::defaults` (lowest) → `Toml::file` → `Env::prefixed` → CLI overrides (highest, merged by caller).

---

### `crates/periphore-ipc/src/server.rs` — service, event-driven

**Pattern source:** `01-RESEARCH.md` Pattern 6 (lines 477–542)

**Unix domain socket server pattern:**
```rust
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use std::path::Path;
use std::fs;

pub async fn serve(
    socket_path: &Path,
    cmd_tx: mpsc::Sender<IpcCommand>,
) -> std::io::Result<()> {
    // Remove stale socket from previous unclean shutdown (Pitfall 2)
    let _ = fs::remove_file(socket_path);

    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(socket_path)?;

    // Set permissions to 0600 immediately after bind (Pitfall 3, D-17)
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
    let (reader_half, writer_half) = stream.into_split();
    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();

    while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
        match serde_json::from_str::<IpcRequest>(line.trim()) {
            Ok(req) => {
                tx.send(IpcCommand::from(req)).await.ok();
            }
            Err(e) => {
                tracing::warn!("Bad IPC request: {e}");
                // Never panic on bad IPC input (security: Denial of Service mitigation)
            }
        }
        line.clear();
    }
}
```

**Security note:** Never `.unwrap()` on IPC input parsing. Bad JSON-lines from a client must log a warning and be skipped, not panic the daemon. This prevents a local DoS via malformed IPC traffic.

---

### `crates/periphore-ipc/src/path.rs` — utility, transform

**Pattern source:** `01-RESEARCH.md` Pattern 8 (lines 584–607)

**Platform socket path pattern:**
```rust
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
    // runtime_dir() returns None on macOS (no XDG standard on macOS) — Assumption A3
    let tmp = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(tmp).join("periphore").join("periphore.sock")
}
```

**Open question:** Verify behavior of `directories 6.0` on macOS for `runtime_dir()`. The research documents this as Assumption A3 (medium risk). Add a Wave 0 test that calls `socket_path()` and asserts it ends in `periphore.sock`.

---

### `crates/periphore-ipc/src/lib.rs` — service, event-driven

**Pattern source:** `01-RESEARCH.md` Pattern 6 + `ARCHITECTURE.md` §4 (channel topology)

**IPC lib.rs pub API pattern:**
```rust
// periphore-ipc/src/lib.rs
// Public surface: serve() function and IpcCommand type for daemon channel communication.

mod server;
pub mod path;

pub use server::serve;

use periphore_protocol::{IpcRequest, InputEvent, Edge};

/// Commands sent from IPC layer to daemon's router via mpsc channel.
/// IPC owns transport; daemon owns routing (ARCHITECTURE.md §Responsibility Map).
#[derive(Debug)]
pub enum IpcCommand {
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

impl From<IpcRequest> for IpcCommand {
    fn from(req: IpcRequest) -> Self {
        match req {
            IpcRequest::GetStatus                    => Self::GetStatus,
            IpcRequest::ListPeers                    => Self::ListPeers,
            IpcRequest::GetTopology                  => Self::GetTopology,
            IpcRequest::AcceptFingerprint { fingerprint } => Self::AcceptFingerprint { fingerprint },
            IpcRequest::RejectFingerprint { fingerprint } => Self::RejectFingerprint { fingerprint },
            IpcRequest::ReloadConfig                 => Self::ReloadConfig,
            IpcRequest::InjectInputEvent { event }   => Self::InjectInputEvent { event },
            IpcRequest::SimulateEdgeCross { edge, position } => Self::SimulateEdgeCross { edge, position },
            IpcRequest::GetState                     => Self::GetState,
            IpcRequest::GetPendingVerifications      => Self::GetPendingVerifications,
            IpcRequest::GetIdenticon { fingerprint } => Self::GetIdenticon { fingerprint },
            IpcRequest::GetWordPhrase { fingerprint } => Self::GetWordPhrase { fingerprint },
        }
    }
}
```

---

### `crates/periphored/src/main.rs` — service, event-driven

**Pattern source:** `01-RESEARCH.md` Patterns 7+9 (lines 549–637) + `ARCHITECTURE.md` §4 (task structure, lines 132–148)

**Daemon main.rs pattern:**
```rust
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

    // Initialize tracing — library crates use tracing:: macros; only daemon initializes subscriber
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let config = periphore_config::load(args.config.as_deref())?;

    // Channel: IPC → daemon router (bounded, control messages)
    let (ipc_cmd_tx, mut ipc_cmd_rx) = tokio::sync::mpsc::channel::<periphore_ipc::IpcCommand>(64);

    let socket_path = periphore_ipc::path::socket_path();

    // Signal handlers (D-29)
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sighup  = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())?;

    let mut tasks = tokio::task::JoinSet::new();

    // Spawn IPC server task
    let ipc_path = socket_path.clone();
    tasks.spawn(async move {
        periphore_ipc::serve(&ipc_path, ipc_cmd_tx).await
            .map_err(|e| anyhow::anyhow!("IPC server error: {e}"))
    });

    // Main daemon loop: signal handling + IPC command dispatch
    loop {
        tokio::select! {
            _ = sigterm.recv() => {
                tracing::info!("SIGTERM received — shutting down");
                break;
            }
            _ = sighup.recv() => {
                tracing::info!("SIGHUP received — reloading config");
                // TODO Phase 4: live config reload
            }
            cmd = ipc_cmd_rx.recv() => {
                match cmd {
                    Some(periphore_ipc::IpcCommand::GetStatus) => {
                        tracing::debug!("IPC GetStatus");
                        // TODO: send IpcResponse::Status back to client
                    }
                    Some(other) => {
                        tracing::debug!("IPC command: {other:?}");
                    }
                    None => {
                        tracing::warn!("IPC command channel closed");
                        break;
                    }
                }
            }
            result = tasks.join_next() => {
                if let Some(Err(e)) = result {
                    tracing::error!("Task failed: {e}");
                }
            }
        }
    }

    // Clean shutdown: remove IPC socket (D-18, D-29)
    let _ = std::fs::remove_file(&socket_path);
    tracing::info!("periphored shutdown complete");
    Ok(())
}
```

**Note:** `anyhow` is used in the binary entry point only. Library crates use `thiserror` for typed errors. (RESEARCH.md Anti-Patterns)

---

### `crates/periphore/src/main.rs` — utility, request-response (stub)

**Pattern source:** `WORKSPACE-PATTERNS.md` §4 (uv pattern: thin entry calls library) + `01-RESEARCH.md` Pattern 9

**CLI entry stub pattern:**
```rust
use clap::Parser;

/// Periphore input sharing CLI
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // Subcommands to be implemented in Phase 5 via periphore-cli library
}

fn main() -> anyhow::Result<()> {
    let _args = Args::parse();
    // Phase 5: periphore_cli::run(args)
    println!("periphore: CLI not yet implemented. Run `periphored` to start the daemon.");
    Ok(())
}
```

---

### Stub crates — `periphore-identity`, `periphore-core`, `periphore-net`, `periphore-capture`, `periphore-inject`, `periphore-cli`

**Pattern source:** `WORKSPACE-PATTERNS.md` §5

**Minimal stub `src/lib.rs` pattern** (identical for all 6 stubs):
```rust
// This crate is a stub. Implementation is deferred to a later phase.
// See ROADMAP.md for the phase assignment.
```

Each stub crate's `Cargo.toml` follows the standard per-crate pattern (no `[lib] doctest/test` override needed for stubs — only thin foundational crates get that).

**Exception:** `periphore-identity` and `periphore-core` DO get `[lib] doctest = false test = false` per D-07 (thin foundational crates). `periphore-cli` does NOT — it is a library that will have tests in Phase 5.

---

### `crates/periphore-ipc/tests/socket.rs` — test, event-driven

**Pattern source:** `01-RESEARCH.md` §Validation Architecture (lines 950–975)

**IPC integration test structure:**
```rust
// tests/socket.rs — integration tests for IPC socket lifecycle
// Run with: cargo test -p periphore-ipc

#[cfg(test)]
mod ipc_tests {
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    // Tests to implement in Wave 0:
    // socket_creates — daemon creates socket at platform path on startup (IPC-01)
    // socket_removed_on_shutdown — socket removed on clean daemon shutdown (IPC-01)
    // get_status_response — GetStatus returns Status response over socket (IPC-02)
    // inject_input_no_peer — InjectInputEvent accepted without network peer (IPC-02)
}
```

---

### `crates/periphore-config/tests/config.rs` — test, CRUD

**Pattern source:** `01-RESEARCH.md` §Validation Architecture (lines 950–975)

**Config test structure:**
```rust
// tests/config.rs — config layering and no-write invariant tests
// Run with: cargo test -p periphore-config

#[cfg(test)]
mod config_tests {
    // Tests to implement in Wave 0:
    // defaults_load_without_file — Config::default() deserializes via Figment
    // env_overrides_file — PERIPHORE_* env vars take precedence over TOML
    // file_overrides_defaults — TOML file values override compiled defaults
    // no_serialize_impl — compile-time: Config has no Serialize impl (verified by cargo build)
}
```

---

## Shared Patterns

### Shared: Error Handling in Library Crates

**Apply to:** `periphore-protocol`, `periphore-config`, `periphore-ipc`, and all future library crates
**Source:** `01-RESEARCH.md` §Anti-Patterns + Standard Stack

```rust
// In library crates: use thiserror for typed errors
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("serialization failed: {0}")]
    Serialize(#[from] postcard::Error),

    #[error("deserialization failed: {0}")]
    Deserialize(#[from] postcard::Error),
}

// In binary entry points (periphored, periphore): use anyhow for ergonomic error propagation
// fn main() -> anyhow::Result<()> { ... }
```

### Shared: Tracing/Logging Discipline

**Apply to:** All crates
**Source:** `CLAUDE.md` + `01-RESEARCH.md` §Project Constraints

```rust
// Library crates: use tracing macros only, never initialize a subscriber
tracing::debug!("IPC command received: {cmd:?}");
tracing::warn!("Bad IPC request: {e}");
tracing::error!("IPC accept error: {e}");
tracing::info!("SIGTERM received — shutting down");

// Only the daemon binary (periphored/src/main.rs) initializes the subscriber:
let subscriber = tracing_subscriber::FmtSubscriber::builder()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .finish();
tracing::subscriber::set_global_default(subscriber)?;
```

### Shared: Platform Guard for Unix-Specific Code

**Apply to:** `periphore-ipc`, `periphore-capture`, `periphore-inject`, `periphored`
**Source:** `CLAUDE.md` + `WORKSPACE-PATTERNS.md` §7

```rust
// Socket permissions, Unix signal handling, etc.
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(socket_path, fs::Permissions::from_mode(0o600))?;
}

// Signal handling — only available on Unix
#[cfg(unix)]
let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;
```

### Shared: Lint Compliance

**Apply to:** All crates — enforced by `[workspace.lints]`
**Source:** `01-RESEARCH.md` Pattern 1 + `WORKSPACE-PATTERNS.md` §1

Every crate `Cargo.toml` must contain exactly:
```toml
[lints]
workspace = true
```

No per-crate `#![deny(...)]` attributes. No `#![allow(...)]` attributes except to suppress workspace-level pedantic lints that are genuinely inappropriate for that file. Workspace lints (`pedantic = "warn"`, `unsafe_code = "warn"`, `unreachable_pub = "warn"`) are the single source of truth.

### Shared: Internal Dependency References

**Apply to:** All crate `Cargo.toml` files
**Source:** `WORKSPACE-PATTERNS.md` §6

NEVER write bare path refs inside crate `Cargo.toml`:
```toml
# WRONG — bypasses workspace dep management:
periphore-protocol = { path = "../periphore-protocol" }

# CORRECT — always reference via workspace:
periphore-protocol = { workspace = true }
```

---

## No Analog Found

All Phase 1 files are new. No existing Rust source code exists in the repository. The table below identifies files where the research documents provide limited pattern guidance, requiring the implementer to exercise judgment:

| File | Role | Data Flow | Reason for Limited Guidance |
|------|------|-----------|------------------------------|
| `crates/periphore-ipc/tests/socket.rs` | test | event-driven | Integration test structure for Unix socket lifecycle not fully specified in research — test names defined but internals need implementation judgment |
| `crates/periphore-config/tests/config.rs` | test | CRUD | Figment test patterns not shown in research — standard Rust `#[test]` with temp files |
| `crates/periphore-ipc/src/lib.rs` (response routing) | service | event-driven | Response-to-client write path partially specified — implementer must design the `(IpcCommand → IpcResponse → write back to stream)` channel plumbing |

---

## Pattern Sources

All patterns are sourced from research documents, not existing source code. Listed in priority order:

| Source File | Authority Level | Used For |
|-------------|-----------------|----------|
| `.planning/phases/01-workspace-protocol-foundation/01-RESEARCH.md` | PRIMARY — all verified patterns | All implementation patterns; 9 explicit code patterns |
| `.planning/phases/01-workspace-protocol-foundation/01-CONTEXT.md` | PRIMARY — locked decisions D-01 to D-31 | File list, feature gating, stub list, invariants |
| `.planning/research/WORKSPACE-PATTERNS.md` | HIGH — uv/typst production references | Workspace Cargo.toml structure, crate layout, binary separation |
| `.planning/research/ARCHITECTURE.md` | HIGH — system design | Channel topology, dependency graph, IPC design |
| `.planning/research/STACK.md` | HIGH — library selection rationale | Dependency choices, version rationale |
| `.planning/research/PITFALLS.md` | HIGH — implementation landmines | Stale socket cleanup, permissions, Figment order, postcard feature |
| `/prek.toml` | Structural reference | Pre-commit hooks in effect (commitizen, end-of-file-fixer, trailing-whitespace) |
| `/cz.toml` | Structural reference | Commit format: conventional commits, `version_provider = "cargo"` |

---

## Metadata

**Analog search scope:** Entire repository (`/Users/spencersr/src/github/whardier/periphore/**/*.rs`)
**Rust files found:** 0 (confirmed greenfield)
**Pattern extraction date:** 2026-04-22
**Toolchain confirmed:** Rust 1.95.0, Edition 2024 stable
**Development platform:** macOS Darwin 25.4.0 (XDG runtime dir not available; `$TMPDIR` path used for IPC socket)
