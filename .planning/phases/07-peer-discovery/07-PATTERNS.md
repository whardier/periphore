# Phase 7: Peer Discovery - Pattern Map

**Mapped:** 2026-04-28
**Files analyzed:** 18 (new/modified files)
**Analogs found:** 18 / 18

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/periphore-discovery/Cargo.toml` | config | -- | `crates/periphore-net/Cargo.toml` | exact |
| `crates/periphore-discovery/src/lib.rs` | service | event-driven | `crates/periphore-net/src/lib.rs` + `manager.rs` | exact |
| `crates/periphore-discovery/src/error.rs` | utility | -- | `crates/periphore-net/src/error.rs` | exact |
| `crates/periphore-discovery/src/mdns.rs` | service | event-driven | `crates/periphore-net/src/manager.rs` (spawn_listener) | role-match |
| `crates/periphore-discovery/src/probe.rs` | service | request-response | `crates/periphore-net/src/manager.rs` (spawn_connector) | role-match |
| `crates/periphore-discovery/src/list.rs` | model | CRUD | `crates/periphore-net/src/manager.rs` (pending map) | role-match |
| `crates/periphore-discovery/tests/integration.rs` | test | -- | `crates/periphore-net/tests/integration.rs` | exact |
| `Cargo.toml` (workspace root) | config | -- | `Cargo.toml` (self) | exact |
| `crates/periphore-config/src/schema.rs` | model | -- | `crates/periphore-config/src/schema.rs` (self) | exact |
| `crates/periphore-protocol/src/ipc.rs` | model | -- | `crates/periphore-protocol/src/ipc.rs` (self) | exact |
| `crates/periphore-ipc/src/lib.rs` | service | request-response | `crates/periphore-ipc/src/lib.rs` (self) | exact |
| `crates/periphored/src/main.rs` | controller | event-driven | `crates/periphored/src/main.rs` (self) | exact |
| `crates/periphore-cli/src/cli.rs` | controller | -- | `crates/periphore-cli/src/cli.rs` (self, Trust pattern) | exact |
| `crates/periphore-cli/src/lib.rs` | controller | request-response | `crates/periphore-cli/src/lib.rs` (self) | exact |
| `crates/periphore-cli/src/commands/peers/mod.rs` | module | -- | `crates/periphore-cli/src/commands/mod.rs` | exact |
| `crates/periphore-cli/src/commands/peers/discovered.rs` | controller | request-response | `crates/periphore-cli/src/commands/status.rs` | exact |
| `crates/periphore-cli/src/commands/peers/pending.rs` | controller | request-response | `crates/periphore-cli/src/commands/status.rs` | exact |
| `crates/periphore-cli/src/commands/mod.rs` (modify) | module | -- | `crates/periphore-cli/src/commands/mod.rs` (self) | exact |

## Pattern Assignments

### `crates/periphore-discovery/Cargo.toml` (config)

**Analog:** `crates/periphore-net/Cargo.toml`

**Full Cargo.toml pattern** (lines 1-30):
```toml
[package]
name = "periphore-net"
version.workspace    = true
edition.workspace    = true
authors.workspace    = true
license.workspace    = true
repository.workspace = true
publish.workspace    = true

[lints]
workspace = true

[dependencies]
periphore-protocol = { workspace = true }
periphore-identity = { workspace = true }
periphore-trust    = { workspace = true }
periphore-config   = { workspace = true }
periphore-core     = { workspace = true }
tokio              = { workspace = true }
tokio-util         = { workspace = true }
bytes              = { workspace = true }
serde              = { workspace = true }
thiserror          = { workspace = true }
tracing            = { workspace = true }
postcard           = { workspace = true }
futures-util       = { workspace = true }
anyhow             = { workspace = true }

[dev-dependencies]
tempfile           = { workspace = true }
```

**Note:** Replace `name = "periphore-net"` with `name = "periphore-discovery"`. Dependencies will differ: add `mdns-sd = { workspace = true }`, keep `periphore-net`, `periphore-config`, `periphore-protocol`, `periphore-identity`, `tokio`, `tokio-util`, `thiserror`, `tracing`, `anyhow`, `futures-util`. Remove crates not needed (e.g., `periphore-core`, `periphore-trust`, `bytes`, `postcard`, `serde`).

---

### `crates/periphore-discovery/src/lib.rs` (service, event-driven)

**Analog:** `crates/periphore-net/src/lib.rs` (module structure) + `crates/periphore-net/src/manager.rs` (service struct pattern)

**Module declarations pattern** (`crates/periphore-net/src/lib.rs`, lines 1-28):
```rust
//! periphore-net: TCP peer connections, handshake, and connection lifecycle.

mod error;
pub mod codec;
mod event;
pub mod connection;
pub mod handshake;
mod manager;

pub use error::NetError;
pub use event::PeerEvent;
pub use connection::{ActiveConn, ConnectionControl, HandshakeResult, PendingPeer};
pub use codec::MAX_FRAME_LENGTH;
pub use handshake::PROTOCOL_VERSION;
pub use manager::ConnectionManager;

/// Default TCP port for peer connections (IANA unassigned, D-08).
pub const DEFAULT_PORT: u16 = 7888;
```

**Service struct + constructor pattern** (`crates/periphore-net/src/manager.rs`, lines 46-68):
```rust
pub struct ConnectionManager {
    event_tx: mpsc::Sender<PeerEvent>,
    peer_tokens: HashMap<String, CancellationToken>,
    pending: Arc<std::sync::Mutex<HashMap<String, PendingPeer>>>,
    active: HashMap<String, ActiveConn>,
}

impl ConnectionManager {
    pub fn new(event_tx: mpsc::Sender<PeerEvent>) -> Self {
        Self {
            event_tx,
            peer_tokens: HashMap::new(),
            pending: Arc::new(std::sync::Mutex::new(HashMap::new())),
            active: HashMap::new(),
        }
    }
```

**Spawn-into-JoinSet pattern** (`crates/periphore-net/src/manager.rs`, lines 78-88):
```rust
    pub fn spawn_listener(
        &mut self,
        tasks: &mut JoinSet<anyhow::Result<()>>,
        bind_addr: SocketAddr,
        identity: Arc<IdentityStore>,
        trust_store: Arc<RwLock<TrustStore>>,
    ) {
        let event_tx = self.event_tx.clone();
        let pending = Arc::clone(&self.pending);

        tasks.spawn(async move {
```

**List snapshot pattern** (`crates/periphore-net/src/manager.rs`, lines 461-474):
```rust
    pub fn pending_list(&self) -> Vec<PendingPeerInfo> {
        let guard = self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard
            .values()
            .map(|p| PendingPeerInfo {
                fingerprint: p.fingerprint_hex.clone(),
                identicon: p.identicon.clone(),
                word_phrase: p.word_phrase.clone(),
            })
            .collect()
    }
```

---

### `crates/periphore-discovery/src/error.rs` (utility)

**Analog:** `crates/periphore-net/src/error.rs`

**Error enum pattern** (lines 1-43):
```rust
//! periphore-net error types.

use thiserror::Error;

/// Errors from TCP peer connection operations.
#[derive(Debug, Error)]
pub enum NetError {
    /// Underlying TCP I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// postcard serialization failure.
    #[error("encode error: {0}")]
    Encode(String),

    /// postcard deserialization failure.
    #[error("decode error: {0}")]
    Decode(String),

    /// Peer closed the connection before handshake completed.
    #[error("connection closed by peer")]
    ConnectionClosed,

    /// Received an unexpected message type at a given handshake step.
    #[error("unexpected message during handshake: {0}")]
    UnexpectedMessage(String),

    /// Protocol version mismatch (local vs remote).
    #[error("protocol version mismatch: local={expected}, remote={got}")]
    ProtocolVersion { expected: u32, got: u32 },

    /// Internal error unrelated to the network protocol (e.g. lock poisoning).
    #[error("internal error: {0}")]
    Internal(String),
}
```

**Apply to `DiscoveryError`:** Replace variants with discovery-specific ones: `MdnsInit(String)`, `MdnsBrowse(String)`, `MdnsRegister(String)`, `Io(#[from] std::io::Error)`, `Internal(String)`.

---

### `crates/periphore-discovery/src/mdns.rs` (service, event-driven)

**Analog:** `crates/periphore-net/src/manager.rs` (spawn_listener pattern)

**Async task with CancellationToken in select! pattern** (`crates/periphore-net/src/manager.rs`, lines 264-424):
```rust
        tasks.spawn(async move {
            // ... setup ...
            loop {
                // ... main loop body ...

                // T-6-05: Check cancellation before sleeping
                tokio::select! {
                    _ = token.cancelled() => {
                        tracing::info!(peer = %peer_key, "connector task cancelled");
                        return Ok(());
                    }
                    _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {}
                }
            }
        });
```

**Accept loop with error-continue pattern** (`crates/periphore-net/src/manager.rs`, lines 88-233):
```rust
        tasks.spawn(async move {
            let listener = TcpListener::bind(bind_addr)
                .await
                .map_err(|e| anyhow::anyhow!("TCP listener bind error on {bind_addr}: {e}"))?;
            tracing::info!(addr = %bind_addr, "TCP peer listener bound");

            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        // ... handle connection ...
                    }
                    Err(e) => {
                        // Mirror server.rs pattern: log and continue
                        tracing::error!(error = %e, "TCP accept error");
                    }
                }
            }
        });
```

---

### `crates/periphore-discovery/src/probe.rs` (service, request-response)

**Analog:** `crates/periphore-net/src/manager.rs` (spawn_connector backoff loop)

**Backoff + CancellationToken loop pattern** (`crates/periphore-net/src/manager.rs`, lines 264-424):
```rust
        tasks.spawn(async move {
            let host = peer_config.host.as_deref().unwrap_or("127.0.0.1").to_owned();
            let port = peer_config.port.unwrap_or(DEFAULT_PORT);
            let addr = format!("{host}:{port}");

            let mut delay_ms = BACKOFF_INITIAL_MS;

            loop {
                match TcpStream::connect(&addr).await {
                    Ok(stream) => {
                        if let Err(e) = stream.set_nodelay(true) {
                            tracing::error!(addr = %addr, error = %e,
                                "TCP_NODELAY failed -- dropping connection, will retry");
                        } else {
                            // ... handshake ...
                        }
                    }
                    Err(e) => {
                        tracing::info!(addr = %addr, error = %e, delay_ms,
                            "peer connection failed -- retrying");
                    }
                }

                tokio::select! {
                    _ = token.cancelled() => {
                        tracing::info!(peer = %peer_key, "connector task cancelled");
                        return Ok(());
                    }
                    _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {}
                }
                delay_ms = (delay_ms * 2).min(BACKOFF_CAP_MS);
            }
        });
```

**TCP_NODELAY immediately after connect** (`crates/periphore-net/src/manager.rs`, lines 278-286):
```rust
                    Ok(stream) => {
                        // D-19 HARD REQUIREMENT: TCP_NODELAY immediately after connect,
                        // before any other socket operation.
                        if let Err(e) = stream.set_nodelay(true) {
                            tracing::error!(
                                addr = %addr,
                                error = %e,
                                "TCP_NODELAY failed -- dropping connection, will retry"
                            );
```

---

### `crates/periphore-discovery/src/list.rs` (model, CRUD)

**Analog:** `crates/periphore-net/src/manager.rs` (pending HashMap + lock pattern)

**Arc<Mutex<HashMap>> shared state pattern** (`crates/periphore-net/src/manager.rs`, lines 52, 65-66):
```rust
    /// Pending peers awaiting user acceptance -- keyed by fingerprint_hex.
    /// Shared with spawned connector/acceptor tasks via Arc.
    pending: Arc<std::sync::Mutex<HashMap<String, PendingPeer>>>,
```

**Lock acquisition with poison recovery** (`crates/periphore-net/src/manager.rs`, lines 164, 206-207, 462-465):
```rust
    let mut guard = pending.lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(fingerprint_hex.clone(), PendingPeer { ... });
```

**Snapshot method returning protocol types** (`crates/periphore-net/src/manager.rs`, lines 461-474):
```rust
    pub fn pending_list(&self) -> Vec<PendingPeerInfo> {
        let guard = self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard
            .values()
            .map(|p| PendingPeerInfo {
                fingerprint: p.fingerprint_hex.clone(),
                identicon: p.identicon.clone(),
                word_phrase: p.word_phrase.clone(),
            })
            .collect()
    }
```

---

### `crates/periphore-discovery/tests/integration.rs` (test)

**Analog:** `crates/periphore-net/tests/integration.rs`

**Test file header pattern** (lines 1-11):
```rust
//! Integration tests for periphore-net handshake protocol (NET-01).
//!
//! Tests run fully in-process using OS-assigned ports (port 0) and fabricated
//! IdentityStore/TrustStore instances. No external infrastructure required.
//!
//! Requirements covered:
//! - NET-01 SC1: trusted peer handshake completes with HandshakeResult::Trusted
//! - ...
```

**Test helper + setup pattern** (lines 13-47):
```rust
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};

use periphore_identity::IdentityStore;
use periphore_net::{codec, handshake, connection::HandshakeResult, NetError, PROTOCOL_VERSION};
use periphore_trust::TrustStore;

const SEED_A: [u8; 32] = [0u8; 32];
const SEED_B: [u8; 32] = [1u8; 32];

fn make_identity(seed: [u8; 32]) -> (IdentityStore, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("key");
    std::fs::write(&path, seed).unwrap();
    let store = IdentityStore::load_or_create(&path).unwrap();
    (store, dir)
}
```

**Async test with timeout pattern** (lines 139-173):
```rust
#[tokio::test]
async fn handshake_trusted_peer() {
    // ... setup ...
    let timeout_dur = Duration::from_secs(5);
    let init_result = tokio::time::timeout(timeout_dur, init_rx)
        .await
        .expect("initiator timed out")
        .expect("initiator channel closed");
    // ... assertions ...
}
```

---

### `Cargo.toml` workspace root (modify)

**Analog:** Self (current `Cargo.toml`)

**Workspace member pattern** (line 3):
```toml
members = ["crates/*"]
```
Note: Uses glob `crates/*` so new crate directory is automatically included. No modification to `members` needed.

**Workspace dependency pattern** (lines 16-27):
```toml
[workspace.dependencies]
periphore-protocol = { path = "crates/periphore-protocol", version = "0.1.0" }
periphore-config   = { path = "crates/periphore-config",   version = "0.1.0" }
periphore-net      = { path = "crates/periphore-net",      version = "0.1.0" }
```

**Add these lines to `[workspace.dependencies]`:**
```toml
periphore-discovery = { path = "crates/periphore-discovery", version = "0.1.0" }
mdns-sd             = { version = "0.19", default-features = true }
```

---

### `crates/periphore-config/src/schema.rs` (modify, model)

**Analog:** Self -- existing config struct patterns

**Config struct with #[serde(default)] and manual Default impl** (lines 1-21, 23-49):
```rust
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub daemon:    DaemonConfig,
    #[serde(default)]
    pub logging:   LoggingConfig,
    #[serde(default, rename = "peer")]
    pub peers:     Vec<PeerConfig>,
    // ...
}

#[derive(Debug, Deserialize)]
pub struct DaemonConfig {
    pub socket_path: Option<std::path::PathBuf>,
    pub port: Option<u16>,
    #[serde(default = "default_listen")]
    pub listen: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: None,
            port:        None,
            listen:      true,
        }
    }
}

fn default_listen() -> bool {
    true
}
```

**Add to Config struct:** `#[serde(default)] pub discovery: DiscoveryConfig,`
**New struct follows DaemonConfig pattern** -- `#[derive(Debug, Deserialize)]` (NOT Serialize, per CFG-01), manual `Default` impl, `#[serde(default = "...")]` for fields with non-trivial defaults.

---

### `crates/periphore-protocol/src/ipc.rs` (modify, model)

**Analog:** Self -- existing IpcRequest/IpcResponse/PendingPeerInfo patterns

**IpcRequest variant pattern** (lines 8-38):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum IpcRequest {
    GetStatus,
    // ...
    GetPendingVerifications,
    // ...
}
```
**Add:** `GetDiscoveredPeers,` (no fields, like `GetPendingVerifications`)

**Info struct pattern** (`PendingPeerInfo`, lines 45-50):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPeerInfo {
    pub fingerprint:  String,
    pub identicon:    String,
    pub word_phrase:  Vec<String>,
}
```
**Add `DiscoveredPeerInfo` adjacent** with same derive macros.

**IpcResponse variant pattern** (lines 64-67):
```rust
    PendingPeers {
        peers: Vec<PendingPeerInfo>,
    },
```
**Add:** `DiscoveredPeers { peers: Vec<DiscoveredPeerInfo> },`

---

### `crates/periphore-ipc/src/lib.rs` (modify, service)

**Analog:** Self -- existing IpcCommand variants

**IpcCommand variant pattern** (lines 59-61):
```rust
    GetPendingVerifications {
        responder: oneshot::Sender<IpcResponse>,
    },
```
**Add:** `GetDiscoveredPeers { responder: oneshot::Sender<IpcResponse> },`

**from_request_with_responder arm pattern** (lines 107-109):
```rust
            IpcRequest::GetPendingVerifications => {
                Self::GetPendingVerifications { responder }
            }
```
**Add:** `IpcRequest::GetDiscoveredPeers => Self::GetDiscoveredPeers { responder },`

---

### `crates/periphored/src/main.rs` (modify, controller)

**Analog:** Self -- existing select! loop and IPC dispatch patterns

**Channel + manager setup pattern** (lines 139-141):
```rust
    let (net_event_tx, mut net_event_rx) = tokio::sync::mpsc::channel::<periphore_net::PeerEvent>(64);
    let mut conn_mgr = periphore_net::ConnectionManager::new(net_event_tx);
```

**Conditional spawn pattern** (lines 144-157):
```rust
    if config.daemon.listen {
        let port = config.daemon.port.unwrap_or(periphore_net::DEFAULT_PORT);
        let bind_addr: std::net::SocketAddr = format!("0.0.0.0:{port}")
            .parse()
            .expect("valid socket address");
        conn_mgr.spawn_listener(
            &mut tasks,
            bind_addr,
            std::sync::Arc::clone(&identity),
            std::sync::Arc::clone(&trust_store),
        );
        tracing::info!(port, "TCP listener started");
    }
```

**Net event select! arm pattern** (lines 223-249):
```rust
            net_event = net_event_rx.recv() => {
                match net_event {
                    Some(periphore_net::PeerEvent::PeerPending { fingerprint, identicon, word_phrase }) => {
                        tracing::warn!("unknown peer pending verification ...");
                    }
                    Some(periphore_net::PeerEvent::PeerConnected { peer_id }) => {
                        tracing::info!(peer_id = %peer_id, "peer connected and trusted");
                    }
                    None => {
                        tracing::warn!("net event channel closed");
                    }
                }
            }
```

**IPC dispatch arm pattern** (lines 308-312):
```rust
                    Some(IpcCommand::GetPendingVerifications { responder }) => {
                        tracing::debug!("IPC: GetPendingVerifications");
                        let peers = conn_mgr.pending_list();
                        let _ = responder.send(IpcResponse::PendingPeers { peers });
                    }
```

**send_ok fallthrough pattern** (lines 464-483):
```rust
fn send_ok(cmd: IpcCommand) {
    match cmd {
        IpcCommand::ListPeers { responder } => {
            let _ = responder.send(IpcResponse::Peers { peers: vec![] });
        }
        // ...
        _ => {}
    }
}
```

---

### `crates/periphore-cli/src/cli.rs` (modify, controller)

**Analog:** Self -- `Trust` subcommand group pattern

**Subcommand group pattern** (lines 36-41):
```rust
    /// Manage peer trust (fingerprint acceptance).
    Trust {
        #[command(subcommand)]
        action: TrustAction,
    },
```

**Sub-action enum pattern** (lines 44-54):
```rust
/// Sub-actions for `periphore trust`.
#[derive(Subcommand, Debug)]
pub enum TrustAction {
    /// Accept a peer's fingerprint and add it to the trust cache.
    Accept {
        /// 64-character hex fingerprint to trust.
        fingerprint: String,
    },
}
```

**Add `Peers` subcommand group and `PeersAction` enum following the same patterns.**

---

### `crates/periphore-cli/src/lib.rs` (modify, controller)

**Analog:** Self -- dispatch pattern

**Match dispatch pattern** (lines 23-31):
```rust
    match cli.command {
        cli::Commands::Status   => commands::status::run(&socket_path).await,
        cli::Commands::Topology => commands::topology::run(&socket_path).await,
        cli::Commands::Trust { action } => match action {
            cli::TrustAction::Accept { fingerprint } => {
                commands::trust::run_accept(&socket_path, &fingerprint).await
            }
        },
    }
```

**Add:** `cli::Commands::Peers { action } => match action { ... }`

---

### `crates/periphore-cli/src/commands/peers/mod.rs` (new, module)

**Analog:** `crates/periphore-cli/src/commands/mod.rs`

**Module declaration pattern** (lines 1-5):
```rust
//! Subcommand handlers for the `periphore` CLI.
//!
//! Each module exposes a single `run(socket_path: &Path) -> anyhow::Result<()>` function.

pub(crate) mod status;
pub(crate) mod topology;
pub(crate) mod trust;
```

---

### `crates/periphore-cli/src/commands/peers/discovered.rs` (new, controller, request-response)

**Analog:** `crates/periphore-cli/src/commands/status.rs`

**Full CLI command handler pattern** (lines 1-48):
```rust
//! Handler for `periphore status`.
//!
//! Sends [`IpcRequest::GetStatus`] to the daemon and prints the running state
//! and identity fingerprint to stdout.

use std::path::Path;

use periphore_protocol::{IpcRequest, IpcResponse};

use crate::client::ipc_request;

/// Run the `status` subcommand.
///
/// Connects to the daemon and prints whether it is running and its fingerprint.
///
/// # Errors
///
/// Returns an error if the daemon is not running or the IPC call fails.
pub(crate) async fn run(socket_path: &Path) -> anyhow::Result<()> {
    let response = ipc_request(socket_path, IpcRequest::GetStatus).await?;
    match response {
        IpcResponse::Status { running, fingerprint } => {
            println!("Daemon:      {}", if running { "running" } else { "not running" });
            match &fingerprint {
                Some(fp) => println!("Fingerprint: {fp}"),
                None     => println!("Fingerprint: (not available)"),
            }
        }
        IpcResponse::Error { message } => {
            anyhow::bail!("daemon error: {message}");
        }
        other => {
            tracing::debug!(?other, "unexpected IPC response for GetStatus");
            anyhow::bail!("unexpected response from daemon");
        }
    }
    Ok(())
}
```

**Apply to `discovered.rs`:** Replace `IpcRequest::GetStatus` with `IpcRequest::GetDiscoveredPeers`, match on `IpcResponse::DiscoveredPeers { peers }`, format as table. Use same error handling arms.

---

### `crates/periphore-cli/src/commands/peers/pending.rs` (new, controller, request-response)

**Analog:** `crates/periphore-cli/src/commands/status.rs`

Same handler pattern as `discovered.rs` above. Use `IpcRequest::GetPendingVerifications`, match on `IpcResponse::PendingPeers { peers }`.

---

### `crates/periphore-cli/src/commands/mod.rs` (modify)

**Analog:** Self

**Current content** (lines 1-6):
```rust
//! Subcommand handlers for the `periphore` CLI.
//!
//! Each module exposes a single `run(socket_path: &Path) -> anyhow::Result<()>` function.

pub(crate) mod status;
pub(crate) mod topology;
pub(crate) mod trust;
```

**Add:** `pub(crate) mod peers;`

---

## Shared Patterns

### Event Enum (DiscoveryEvent)
**Source:** `crates/periphore-net/src/event.rs` (lines 1-34)
**Apply to:** `crates/periphore-discovery/src/lib.rs` (define `DiscoveryEvent` enum)
```rust
//! periphore-net peer events: one-way notifications from ConnectionManager to periphored.

use periphore_core::PeerId;

/// Events emitted by `ConnectionManager` and consumed by the daemon's select! loop.
#[derive(Debug)]
pub enum PeerEvent {
    PeerPending {
        fingerprint: String,
        identicon: String,
        word_phrase: Vec<String>,
    },
    PeerConnected {
        peer_id: PeerId,
    },
    PeerDisconnected {
        peer_id: PeerId,
    },
}
```

### Error Handling (thiserror enum)
**Source:** `crates/periphore-net/src/error.rs` (lines 1-43)
**Apply to:** `crates/periphore-discovery/src/error.rs`
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    // ...
    #[error("internal error: {0}")]
    Internal(String),
}
```

### IPC Client Request/Response
**Source:** `crates/periphore-cli/src/client.rs` (lines 1-59)
**Apply to:** `crates/periphore-cli/src/commands/peers/discovered.rs` and `pending.rs`
```rust
use crate::client::ipc_request;

pub(crate) async fn run(socket_path: &Path) -> anyhow::Result<()> {
    let response = ipc_request(socket_path, IpcRequest::...).await?;
    match response {
        IpcResponse::... { ... } => { /* format output */ }
        IpcResponse::Error { message } => { anyhow::bail!("daemon error: {message}"); }
        other => {
            tracing::debug!(?other, "unexpected IPC response for ...");
            anyhow::bail!("unexpected response from daemon");
        }
    }
    Ok(())
}
```

### IPC Dispatch (oneshot responder)
**Source:** `crates/periphored/src/main.rs` (lines 308-312)
**Apply to:** `crates/periphored/src/main.rs` (new `GetDiscoveredPeers` dispatch arm)
```rust
Some(IpcCommand::GetPendingVerifications { responder }) => {
    tracing::debug!("IPC: GetPendingVerifications");
    let peers = conn_mgr.pending_list();
    let _ = responder.send(IpcResponse::PendingPeers { peers });
}
```

### Workspace Dependency Convention
**Source:** `Cargo.toml` root (lines 16-27)
**Apply to:** All new dependency additions
```toml
# Internal: path + version
periphore-discovery = { path = "crates/periphore-discovery", version = "0.1.0" }
# External: version only
mdns-sd             = { version = "0.19", default-features = true }
```

### Config Struct Convention (Deserialize-only)
**Source:** `crates/periphore-config/src/schema.rs` (lines 1-4)
**Apply to:** New `DiscoveryConfig` struct
```rust
// CRITICAL: Config intentionally does NOT derive Serialize.
// This enforces CFG-01 at compile time: no code path can serialize Config to disk.
```

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none) | -- | -- | All files have strong analogs in the existing codebase |

## Metadata

**Analog search scope:** `crates/periphore-net/`, `crates/periphore-config/`, `crates/periphore-protocol/`, `crates/periphore-ipc/`, `crates/periphore-cli/`, `crates/periphored/`, root `Cargo.toml`
**Files scanned:** 18 source files + 1 integration test
**Pattern extraction date:** 2026-04-28
