# Phase 6: TCP Peering — Pattern Map

**Mapped:** 2026-04-26
**Files analyzed:** 13 (9 new, 4 modified)
**Analogs found:** 12 / 13 (1 has no analog — systemd unit file)

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/periphore-net/src/lib.rs` | utility/pub-use | — | `crates/periphore-ipc/src/lib.rs` | role-match |
| `crates/periphore-net/src/error.rs` | utility/error-type | — | `crates/periphore-trust/src/store.rs` (TrustError) | exact |
| `crates/periphore-net/src/codec.rs` | utility/transform | transform | `crates/periphore-ipc/src/server.rs` (framing analog) | partial |
| `crates/periphore-net/src/handshake.rs` | service | request-response | `crates/periphore-ipc/src/server.rs` (handle_connection) | role-match |
| `crates/periphore-net/src/connection.rs` | model | — | `crates/periphore-trust/src/store.rs` (TrustedPeer struct) | partial |
| `crates/periphore-net/src/manager.rs` | service | event-driven | `crates/periphore-ipc/src/server.rs` (serve + JoinSet) | role-match |
| `crates/periphore-net/src/event.rs` | model/enum | event-driven | `crates/periphore-ipc/src/lib.rs` (IpcCommand enum) | role-match |
| `crates/periphore-net/tests/integration.rs` | test | request-response | `crates/periphore-ipc/tests/socket.rs` | exact |
| `crates/periphored/tests/net_wiring.rs` | test | event-driven | `crates/periphore-ipc/tests/socket.rs` | role-match |
| `contrib/periphored.service` | config | — | (none) | no analog |
| `crates/periphore-net/Cargo.toml` | config | — | `crates/periphored/Cargo.toml` | exact |
| `crates/periphore-config/src/schema.rs` | model | — | `crates/periphore-config/src/schema.rs` (DaemonConfig) | exact (self) |
| `crates/periphore-protocol/src/ipc.rs` | model | — | `crates/periphore-protocol/src/ipc.rs` (IpcResponse) | exact (self) |
| `crates/periphored/src/main.rs` | service/router | event-driven | `crates/periphored/src/main.rs` | exact (self) |
| `crates/periphored/Cargo.toml` | config | — | `crates/periphored/Cargo.toml` | exact (self) |

---

## Pattern Assignments

### `crates/periphore-net/src/lib.rs` (utility, pub-use module root)

**Analog:** `crates/periphore-ipc/src/lib.rs`

**Module structure pattern** (`crates/periphore-ipc/src/lib.rs` lines 1–16):
```rust
//! periphore-ipc: Unix domain socket IPC service for Periphore.
//!
//! Provides:
//! - `serve()`: ...
//! - `path::socket_path()`: ...
//! - `IpcCommand`: ...

mod server;
pub mod path;

pub use server::serve;

use tokio::sync::oneshot;
use periphore_protocol::{Edge, InputEvent, IpcRequest, IpcResponse};
```

**Apply to `lib.rs`:** Declare all internal modules (`mod error; mod codec; mod handshake; mod connection; mod manager; mod event;`), then `pub use` the public surface (`ConnectionManager`, `PeerEvent`, `NetError`, `DEFAULT_PORT`). Add crate-level doc comment describing the crate's responsibility. Note: `periphore-net` has `[lib] test = false` per workspace convention — confirm in Cargo.toml before adding unit tests inside `src/`.

---

### `crates/periphore-net/src/error.rs` (utility, error type)

**Analog:** `crates/periphore-trust/src/store.rs` — `TrustError` enum (lines 9–39)

**Error enum pattern** (`crates/periphore-trust/src/store.rs` lines 9–39):
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrustError {
    #[error("trust cache file is corrupt: {0}")]
    CorruptCacheFile(String),

    #[error("fingerprint conflict for peer '{peer_label}': expected {expected}, got {actual}")]
    FingerprintConflict {
        expected: String,
        actual: String,
        peer_label: String,
    },

    #[error("fingerprint not found in trust cache: {0}")]
    NotFound(String),

    #[error("serialization error: {0}")]
    SerializeError(String),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}
```

**Apply to `error.rs`:** Same pattern — `thiserror::Error` derive, `#[error(...)]` on each variant. NetError variants needed:
- `Io(#[from] std::io::Error)` — wraps all TCP I/O errors
- `Encode(String)` — postcard serialization failure
- `Decode(String)` — postcard deserialization failure
- `ConnectionClosed` — peer closed connection during handshake
- `UnexpectedMessage(String)` — wrong message type at a handshake step
- `FingerprintConflict(String)` — configured fingerprint does not match (calls `check_peer_fingerprint`)
- `ProtocolVersion { expected: u32, got: u32 }` — version mismatch
- `PeerNotFound(String)` — promote_pending called for unknown fingerprint

Do NOT use `anyhow` in `periphore-net`. Library crates use `thiserror`; `anyhow` is daemon-boundary only (see `periphored/Cargo.toml`).

---

### `crates/periphore-net/src/codec.rs` (utility, transform)

**No direct analog in codebase** — the IPC server uses `tokio::io::AsyncBufReadExt` for JSON-lines; the TCP codec uses `LengthDelimitedCodec` + `postcard`. The RESEARCH.md patterns are the primary reference here.

**Key constraints from codebase and RESEARCH.md:**
- Use `LengthDelimitedCodec::builder().max_frame_length(64 * 1024).new_codec()` (security: prevent OOM from malicious length header — Pattern 10 in RESEARCH.md security domain)
- Do NOT use `LengthDelimitedCodec::new()` without max_frame_length
- Always `stream.into_split()` before creating `FramedRead`/`FramedWrite` — never `Framed<TcpStream, ...>` directly (RESEARCH.md Pitfall 2)
- `TCP_NODELAY` must be set BEFORE calling any codec function — set it in the caller (manager.rs) immediately after `connect()`/`accept()`, before passing the stream to codec
- `postcard::to_allocvec` for encode; `postcard::from_bytes` for decode
- Create two separate codec instances (one per split half) — codec may not implement Clone (RESEARCH.md A1)

**split_framed function signature to implement:**
```rust
pub fn split_framed(
    stream: tokio::net::TcpStream,
) -> (
    tokio_util::codec::FramedRead<tokio::net::tcp::OwnedReadHalf, tokio_util::codec::LengthDelimitedCodec>,
    tokio_util::codec::FramedWrite<tokio::net::tcp::OwnedWriteHalf, tokio_util::codec::LengthDelimitedCodec>,
)
```

**encode/decode signatures:**
```rust
pub fn encode_message(msg: &periphore_protocol::PeerMessage) -> Result<bytes::Bytes, crate::error::NetError>
pub fn decode_message(frame: bytes::BytesMut) -> Result<periphore_protocol::PeerMessage, crate::error::NetError>
```

---

### `crates/periphore-net/src/handshake.rs` (service, request-response)

**Analog:** `crates/periphore-ipc/src/server.rs` — `handle_connection` function (lines 73–158)

**Connection handler pattern — sequential async fn in spawned task** (`server.rs` lines 73–110):
```rust
async fn handle_connection(stream: UnixStream, tx: mpsc::Sender<IpcCommand>) {
    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();

    while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
        // ... parse, dispatch, respond
        line.clear();
    }
}
```

**Timeout pattern** (`server.rs` lines 106–132):
```rust
match tokio::time::timeout(std::time::Duration::from_secs(5), resp_rx).await {
    Ok(Ok(response)) => { /* use response */ }
    Ok(Err(_)) => { /* responder dropped */ }
    Err(_) => { /* timeout — send error, break */ break; }
}
```

**Error non-panic pattern** (`server.rs` lines 140–157):
```rust
Err(e) => {
    tracing::warn!("Malformed IPC request (ignored): {e}. Input: {trimmed:?}");
    let response = IpcResponse::Error { message: format!("malformed request: {e}") };
    // write response; never panic
}
```

**Apply to `handshake.rs`:** The `perform_handshake()` function runs in a spawned task (called from `manager.rs`). It is a sequential async function — NOT a poll-based state machine. Use `tokio_stream::StreamExt::next()` for FramedRead reads and `futures::SinkExt::send()` for FramedWrite writes. Apply `tokio::time::timeout` around handshake reads with a sensible timeout (e.g., 5s) to prevent hanging on slow/malicious peers. Never `unwrap()` on peer data — always propagate via `NetError`. The function returns a `HandshakeResult` enum, not a `Result<(), NetError>`.

---

### `crates/periphore-net/src/connection.rs` (model, structs)

**Analog:** `crates/periphore-trust/src/store.rs` — `TrustedPeer` and `TrustCache` structs (lines 42–59)

**Struct definition pattern** (`store.rs` lines 42–59):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedPeer {
    pub fingerprint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

#[derive(Debug)]
pub struct TrustStore {
    cache: TrustCache,
}
```

**Apply to `connection.rs`:** Define `PendingPeer`, `ActiveConn`, `ConnectionState`, `ConnectionControl`, and `HandshakeResult`. These are pure data types with no async logic. `PendingPeer` carries `fingerprint_hex: String`, `identicon: String`, `word_phrase: Vec<String>`, and `promote_tx: mpsc::Sender<ConnectionControl>`. `ConnectionControl` is an enum with `PromoteTrusted` and `Reject` variants. `HandshakeResult` has `Trusted`, `Pending`, and `Rejected` variants carrying the relevant data.

---

### `crates/periphore-net/src/manager.rs` (service, event-driven)

**Analog:** `crates/periphore-ipc/src/server.rs` — `serve()` function + `crates/periphored/src/main.rs` — JoinSet pattern

**Accept loop pattern** (`server.rs` lines 18–62):
```rust
pub async fn serve(socket_path: &Path, cmd_tx: mpsc::Sender<IpcCommand>) -> std::io::Result<()> {
    // setup...
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let tx = cmd_tx.clone();
                tokio::spawn(handle_connection(stream, tx));
            }
            Err(e) => {
                tracing::error!("IPC accept error: {e}");
                // Continue serving; a single accept error should not crash the server.
            }
        }
    }
}
```

**JoinSet spawn pattern** (`periphored/src/main.rs` lines 114–121):
```rust
let mut tasks = tokio::task::JoinSet::new();

let ipc_path = socket_path.clone();
tasks.spawn(async move {
    periphore_ipc::serve(&ipc_path, ipc_cmd_tx)
        .await
        .map_err(|e| anyhow::anyhow!("IPC server error: {e}"))
});
```

**JoinSet join_next pattern** (`periphored/src/main.rs` lines 236–254):
```rust
result = tasks.join_next(), if !tasks.is_empty() => {
    match result {
        Some(Ok(Ok(()))) => { tracing::info!("..."); break; }
        Some(Ok(Err(e))) => { tracing::error!("...: {e}"); break; }
        Some(Err(e)) => { tracing::error!("Task panicked: {e}"); break; }
        None => { /* unreachable */ }
    }
}
```

**Apply to `manager.rs`:** `ConnectionManager` struct holds `event_tx: mpsc::Sender<PeerEvent>`, `peer_tokens: HashMap<String, CancellationToken>`, `pending: HashMap<String, PendingPeer>`, `active: HashMap<String, ActiveConn>`. Methods: `new()`, `spawn_listener()`, `spawn_connector()`, `promote_pending()` (async), `pending_list()`, `cancel_peer()`. `spawn_listener` and `spawn_connector` take `&mut JoinSet<anyhow::Result<()>>` (same type as periphored's tasks) and spawn tasks into it. `TCP_NODELAY` is set immediately after `accept()` or `connect()` — before ANY other operation — per CLAUDE.md hard requirement. Accept loop error handling mirrors `server.rs`: log the error and continue (never abort the loop on a single bad connection).

**CancellationToken usage** — every retry connector task must check the token:
```rust
tokio::select! {
    _ = shutdown.cancelled() => { return; }
    _ = tokio::time::sleep(std::time::Duration::from_millis(delay_ms)) => {}
}
```

---

### `crates/periphore-net/src/event.rs` (model, event enum)

**Analog:** `crates/periphore-ipc/src/lib.rs` — `IpcCommand` enum (lines 27–71)

**Event enum pattern** (`lib.rs` lines 27–71):
```rust
#[derive(Debug)]
pub enum IpcCommand {
    GetStatus {
        responder: oneshot::Sender<IpcResponse>,
    },
    AcceptFingerprint {
        fingerprint: String,
        responder: oneshot::Sender<IpcResponse>,
    },
    // ...
}
```

**Apply to `event.rs`:** `PeerEvent` is a `#[derive(Debug)]` enum sent from `periphore-net` → `periphored` via `mpsc`. Variants needed:
- `PeerPending { fingerprint: String, identicon: String, word_phrase: Vec<String> }` — unknown peer held pending
- `PeerConnected { peer_id: periphore_core::PeerId }` — trusted peer handshake complete
- `PeerDisconnected { peer_id: periphore_core::PeerId }` — established connection dropped

Unlike `IpcCommand`, `PeerEvent` does NOT carry a `oneshot::Sender` — it is a one-way notification from net to daemon. There is no response from daemon back to the event sender. Control in the reverse direction (daemon → net) goes through `ConnectionManager` method calls or the `ConnectionControl` channel in `PendingPeer`.

---

### `crates/periphore-net/tests/integration.rs` (test, integration)

**Analog:** `crates/periphore-ipc/tests/socket.rs` — full integration test file

**Test helper pattern** (`socket.rs` lines 20–58):
```rust
fn temp_socket_path(test_name: &str) -> std::path::PathBuf {
    let tmp = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_owned());
    std::path::PathBuf::from(tmp)
        .join("periphore-test")
        .join(format!("{test_name}-{}.sock", std::process::id()))
}

async fn spawn_test_server(test_name: &str) -> (JoinHandle<...>, JoinHandle<()>, PathBuf) {
    let path = temp_socket_path(test_name);
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<IpcCommand>(16);
    let router = tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await { handle_test_command(cmd); }
    });
    let server_path = path.clone();
    let server = tokio::spawn(async move { periphore_ipc::serve(&server_path, cmd_tx).await });
    tokio::time::sleep(Duration::from_millis(50)).await; // give server time to bind
    (server, router, path)
}
```

**Test structure pattern** (`socket.rs` lines 139–151):
```rust
#[tokio::test]
async fn socket_creates() {
    let (server, router, path) = spawn_test_server("socket_creates").await;
    assert!(path.exists(), "...");
    server.abort();
    router.abort();
    let _ = std::fs::remove_file(&path);
}
```

**Apply to `integration.rs`:** Use `TcpListener::bind("127.0.0.1:0")` (OS-assigned port) instead of a file path helper. Create in-process listener and connector. Use real `IdentityStore` instances loaded from `tempfile::TempDir`. Each test binds its own listener on port 0 to avoid conflicts. Test cases to implement:
1. `handshake_completes_trusted` — both peers have matching trust; assert `PeerEvent::PeerConnected`
2. `handshake_unknown_peer_goes_pending` — empty trust store; assert `PeerEvent::PeerPending` with fingerprint
3. `promote_pending_sends_peer_connected` — call `promote_pending()` after pending event; assert next event is `PeerConnected`
4. `protocol_version_mismatch_drops_connection` — send mismatched version; assert connection closed
5. Codec roundtrip unit test (can be a `#[test]` not `#[tokio::test]`): encode then decode `PeerMessage::Hello`

All async tests use `#[tokio::test]`. Use `tokio::time::timeout` in test assertions to avoid hanging CI.

---

### `crates/periphored/tests/net_wiring.rs` (test, integration)

**Analog:** `crates/periphore-ipc/tests/socket.rs` — spawn_test_server pattern + router verification

**Apply:** This test verifies NET-03 (auto-connect from `[[peer]]` config) and the `GetPendingVerifications` IPC stub promotion. Structure mirrors `socket.rs`: spawn a minimal periphored-like select! loop with `ConnectionManager` wired up, provide a `PeerConfig` with `host = "127.0.0.1"` and an OS-assigned port, assert that `PeerEvent::PeerPending` arrives within timeout.

---

### `crates/periphore-net/Cargo.toml` (config, dependency manifest)

**Analog:** `crates/periphored/Cargo.toml` — workspace dep declaration pattern

**Workspace dep pattern** (`periphored/Cargo.toml` lines 1–31):
```toml
[package]
name = "periphored"
version.workspace    = true
edition.workspace    = true
authors.workspace    = true
license.workspace    = true
repository.workspace = true
publish.workspace    = true

[lints]
workspace = true

[dependencies]
periphore-config   = { workspace = true }
periphore-identity = { workspace = true }
# ...
tokio              = { workspace = true }
anyhow             = { workspace = true }
```

**Apply:** Keep all existing entries in `periphore-net/Cargo.toml`. Add the four new internal crate deps (all use `{ workspace = true }`):
```toml
periphore-identity = { workspace = true }
periphore-trust    = { workspace = true }
periphore-config   = { workspace = true }
periphore-core     = { workspace = true }
```
Also add `postcard` if not already present (check: current Cargo.toml has `tokio-util`, `bytes`, `serde`, `thiserror`, `tracing` — `postcard` is NOT listed; it must be added). Add `futures-util` for `SinkExt`/`StreamExt` if needed (verify at compile time; tokio-util may re-export these).

**Dev-dependencies:** Add `tempfile = { workspace = true }` for integration tests.

---

### `crates/periphore-config/src/schema.rs` (model, schema extension)

**Analog:** `crates/periphore-config/src/schema.rs` — `DaemonConfig` struct (lines 24–30) — extending self

**Existing struct** (`schema.rs` lines 24–30):
```rust
#[derive(Debug, Deserialize, Default)]
pub struct DaemonConfig {
    pub socket_path: Option<std::path::PathBuf>,
    pub port: Option<u16>,
}
```

**serde default function pattern** (from RESEARCH.md Pattern 8):
```rust
#[serde(default = "default_listen")]
pub listen: bool,

fn default_listen() -> bool { true }
```

**Apply:** Add `listen: bool` field to `DaemonConfig` with `#[serde(default = "default_listen")]`. Add `fn default_listen() -> bool { true }` as a private free function in the same file. This field requires a daemon restart to take effect (like `port` and `socket_path`) — Phase 6 adds it to the restart-required check in `reload_config()` in `periphored/src/main.rs`.

---

### `crates/periphore-protocol/src/ipc.rs` (model, enum extension)

**Analog:** `crates/periphore-protocol/src/ipc.rs` — `IpcResponse` enum (lines 41–69) — extending self

**Existing enum pattern** (`ipc.rs` lines 41–69):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum IpcResponse {
    Status { running: bool, fingerprint: Option<String> },
    Peers { peers: Vec<String> },
    Identicon { fingerprint_hex: String, identicon: String },
    WordPhrase { words: Vec<String>, phrase: String },
    Ok,
    Error { message: String },
}
```

**Apply:** Add `PendingPeers { peers: Vec<PendingPeerInfo> }` variant to `IpcResponse`. Add `PendingPeerInfo` struct in the same file (or a new `types` submodule if it grows). The struct needs `#[derive(Debug, Clone, Serialize, Deserialize)]`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPeerInfo {
    pub fingerprint: String,
    pub identicon: String,
    pub word_phrase: Vec<String>,
}
```
The `serde(rename_all = "snake_case", tag = "type")` on `IpcResponse` means `PendingPeers` serializes as `{"type":"pending_peers","peers":[...]}`. Do NOT repurpose `Peers { peers: Vec<String> }` — data shapes differ.

---

### `crates/periphored/Cargo.toml` (config, dependency manifest)

**Analog:** self (lines 1–31) — adding two new workspace deps

**Apply:** Add to `[dependencies]`:
```toml
periphore-net  = { workspace = true }
periphore-core = { workspace = true }
```
No other changes. Both are already declared in the root `Cargo.toml` `[workspace.dependencies]` (lines 23 and 21 respectively).

---

### `crates/periphored/src/main.rs` (service/router, event-driven — extending self)

**Analog:** self — all patterns already present; Phase 6 extends them

**Channel declaration pattern** (lines 111–112):
```rust
let (ipc_cmd_tx, mut ipc_cmd_rx) = mpsc::channel::<IpcCommand>(64);
```

**Task spawn pattern** (lines 114–121):
```rust
let mut tasks = tokio::task::JoinSet::new();
tasks.spawn(async move {
    periphore_ipc::serve(&ipc_path, ipc_cmd_tx)
        .await
        .map_err(|e| anyhow::anyhow!("IPC server error: {e}"))
});
```

**IpcCommand dispatch pattern** (lines 148–233):
```rust
cmd = ipc_cmd_rx.recv() => {
    match cmd {
        Some(IpcCommand::AcceptFingerprint { fingerprint, responder }) => {
            match trust_store.add_trusted(&fingerprint, None, &trust_path) {
                Ok(()) => { let _ = responder.send(IpcResponse::Ok); }
                Err(e) => { let _ = responder.send(IpcResponse::Error { message: ... }); }
            }
        }
        Some(other) => { send_ok(other); }
        None => { break; }
    }
}
```

**reload_config restart-required pattern** (lines 311–318):
```rust
if new_config.daemon.socket_path != current_config.daemon.socket_path {
    tracing::warn!("config field 'daemon.socket_path' changed but requires restart to take effect");
}
if new_config.daemon.port != current_config.daemon.port {
    tracing::warn!("config field 'daemon.port' changed but requires restart to take effect");
}
```

**resolve_identicon free function pattern** (lines 29–35):
```rust
fn resolve_identicon(show_identicon: bool, identity: &periphore_identity::IdentityStore) -> String {
    if show_identicon { identity.identicon() } else { String::new() }
}
```

**Apply to `main.rs`:** Six changes:

1. **macOS SSH check** — add at the very top of `main()`, before any async setup (lines ~38):
```rust
#[cfg(target_os = "macos")]
{
    use std::io::IsTerminal as _;
    if !std::io::stdin().is_terminal() {
        eprintln!(
            "error: periphored must be launched from a local terminal or launchd on macOS.\n\
             Remote SSH launch is not supported on macOS.\n\
             Start the daemon locally, then connect to it via SSH tunnel if needed."
        );
        std::process::exit(1);
    }
}
```

2. **Channel + manager init** — after trust store load, before tasks spawn:
```rust
let (net_event_tx, mut net_event_rx) = mpsc::channel::<periphore_net::PeerEvent>(64);
let mut conn_mgr = periphore_net::ConnectionManager::new(net_event_tx);
let mut focus_sm = periphore_core::FocusStateMachine::new();
```

3. **Spawn listener + connectors** — after manager init, before select! loop. Mirror the `tasks.spawn` pattern already used for IPC.

4. **net_event arm in select!** — add as a new branch alongside existing signal/IPC/JoinSet arms. Follows the same `match` pattern as the IpcCommand arm.

5. **AcceptFingerprint extension** — after `trust_store.add_trusted(&fingerprint, ...)` succeeds, add `let _ = conn_mgr.promote_pending(&fingerprint).await;`

6. **GetPendingVerifications real dispatch** — replace `send_ok(other)` stub with:
```rust
Some(IpcCommand::GetPendingVerifications { responder }) => {
    let peers = conn_mgr.pending_list();
    let _ = responder.send(IpcResponse::PendingPeers { peers });
}
```

7. **daemon.listen in reload_config** — add to the restart-required block:
```rust
if new_config.daemon.listen != current_config.daemon.listen {
    tracing::warn!("config field 'daemon.listen' changed but requires restart to take effect");
}
```

---

### `contrib/periphored.service` (config, systemd unit)

**No analog in codebase.** Use RESEARCH.md Pattern (systemd user unit, lines 783–813) as the template:
```ini
[Unit]
Description=Periphore input sharing daemon
Documentation=https://github.com/whardier/periphore
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=%h/.cargo/bin/periphored
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal
NoNewPrivileges=true

[Install]
WantedBy=default.target
```

Placement: `contrib/periphored.service` at repository root level. Not compiled; not tested. Only a documentation artifact.

---

## Shared Patterns

### `thiserror` Error Enum (library crates only)
**Source:** `crates/periphore-trust/src/store.rs` lines 9–39
**Apply to:** `periphore-net/src/error.rs`
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrustError {  // → NetError
    #[error("...")]
    SomeVariant(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```
Rule: library crates (`periphore-net`, `periphore-protocol`, `periphore-config`) use `thiserror`. Only `periphored` and `periphore` use `anyhow` at the binary boundary.

### `tracing` Event Logging
**Source:** `crates/periphored/src/main.rs` (throughout) and `crates/periphore-trust/src/store.rs` (lines 69, 95, 103, 115)
**Apply to:** All `periphore-net` modules and `periphored/src/main.rs` additions

Conventions observed:
- Structured fields: `tracing::info!(fingerprint = %fp_lower, "message")` — `%` for Display, `?` for Debug
- Error fields: `tracing::error!(error = %e, "message")` — never interpolate errors in the message string
- Levels: `error!` for drop-connection conditions, `warn!` for pending peer + fingerprint conflict, `info!` for connection lifecycle, `debug!` for IPC command receipt

### IpcCommand oneshot Responder Pattern
**Source:** `crates/periphore-ipc/src/lib.rs` lines 27–71 and `crates/periphore-ipc/src/server.rs` lines 95–96
**Apply to:** `periphored/src/main.rs` dispatch for new `GetPendingVerifications`
```rust
// server.rs: create oneshot pair per request
let (resp_tx, resp_rx) = tokio::sync::oneshot::channel::<IpcResponse>();
let cmd = IpcCommand::from_request_with_responder(req, resp_tx);
```
Every `IpcCommand` variant carries its own `oneshot::Sender<IpcResponse>`. The daemon always calls `let _ = responder.send(...)` (ignoring the send error — client may have disconnected).

### JoinSet Task Lifecycle
**Source:** `crates/periphored/src/main.rs` lines 114–121, 236–254
**Apply to:** `periphore-net/src/manager.rs` (spawn_listener, spawn_connector) and `periphored/src/main.rs` extension
```rust
let mut tasks = tokio::task::JoinSet::new();
tasks.spawn(async move { /* ... */ .map_err(|e| anyhow::anyhow!("...{e}")) });
// In select!:
result = tasks.join_next(), if !tasks.is_empty() => {
    match result {
        Some(Ok(Ok(()))) => { /* clean exit */ }
        Some(Ok(Err(e))) => { tracing::error!("...: {e}"); }
        Some(Err(e)) => { tracing::error!("Task panicked: {e}"); }
        None => {}
    }
}
// On shutdown:
tasks.abort_all();
```

### workspace = true Dependency Declaration
**Source:** `crates/periphored/Cargo.toml` lines 17–27, root `Cargo.toml` lines 16–27
**Apply to:** `crates/periphore-net/Cargo.toml` (new internal deps), `crates/periphored/Cargo.toml` (two new deps)

Every dependency — internal crate or external — uses `{ workspace = true }`. Never pin versions in individual crate Cargo.toml files. Declare new workspace-level deps only in the root `Cargo.toml` `[workspace.dependencies]` block if not already present.

Current root workspace deps relevant to Phase 6 (all already present):
- `periphore-identity`, `periphore-trust`, `periphore-config`, `periphore-core`, `periphore-net` — lines 17–23
- `postcard`, `tokio`, `tokio-util`, `bytes`, `thiserror`, `tracing` — lines 31–41

### Test Isolation via Unique Addresses
**Source:** `crates/periphore-ipc/tests/socket.rs` lines 20–25 (unique temp paths)
**Apply to:** `crates/periphore-net/tests/integration.rs`
```rust
// IPC analog uses unique file paths. TCP analog uses port 0:
let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
let addr = listener.local_addr().unwrap(); // OS-assigned port
```
Use `TcpListener::bind("127.0.0.1:0")` in every test — never hardcode a port. Combine with `tokio::time::timeout` around assertions to prevent CI hangs on connection failures.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `contrib/periphored.service` | config | — | No systemd unit files exist in this codebase; use RESEARCH.md pattern directly |
| `crates/periphore-net/src/codec.rs` | utility | transform | No LengthDelimitedCodec + postcard usage exists yet; use RESEARCH.md Pattern 1 + security max_frame_length |

---

## Metadata

**Analog search scope:** `crates/` directory — all Rust source files
**Files scanned:** 12 source files read in full
**Pattern extraction date:** 2026-04-26

**Codebase conventions confirmed:**
- `thiserror` in library crates, `anyhow` at binary boundary
- `{ workspace = true }` for all Cargo.toml deps
- `[lib] test = false` implied — integration tests in `tests/` subdirectory only
- `tokio::task::JoinSet` for task lifecycle (not `Vec<JoinHandle>`)
- Structured `tracing` fields (`%field` for Display, `?field` for Debug)
- `let _ = responder.send(...)` — always ignore oneshot send result
- `tasks.abort_all()` on clean shutdown
- `tempfile` crate for test isolation (already in workspace dev-deps)
