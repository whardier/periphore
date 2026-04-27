# Phase 6: TCP Peering — Research

**Researched:** 2026-04-26
**Domain:** Async TCP peering, protocol framing, exponential backoff, connection lifecycle, systemd
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- D-01: Unknown peer → hold in Pending state (Hello/HelloAck completed, input blocked until user accepts via AcceptFingerprint IPC)
- D-02: Pending notification via `tracing::warn!` with fingerprint hex, identicon, and word-phrase; user runs `periphore trust accept <fp>`
- D-03: GetPendingVerifications IPC must be wired to real implementation in Phase 6 (currently a stub)
- D-04: Fingerprint conflict → `tracing::error!` + drop connection (locked from Phase 3)
- D-05: Auto-connect on startup to all `[[peer]]` entries with `host` set
- D-06: Auto-connect uses exponential backoff (1s→2s→4s→8s→16s→cap 30s)
- D-07: Daemon listens symmetrically (P2P); `daemon.listen` config field (default: true)
- D-08: Default TCP port — planner's discretion from 7700–8000 range (avoid 24800/Synergy)
- D-09: Auto-reconnect on unexpected disconnect with exponential backoff
- D-10: Auto-reconnect applies to outbound connections; inbound: remote side reconnects
- D-11: Peer removed from config → cancel its reconnect loop
- D-12: No `--daemonize` flag; foreground only; nohup/systemd for persistence
- D-13: Document two persistence paths: `nohup periphored &` and systemd user unit
- D-14: Ship `contrib/periphored.service` systemd user unit file
- D-15: macOS SSH detection via `!isatty(0)` → clear error to stderr + exit
- D-16: macOS SSH check is `#[cfg(target_os = "macos")]` only
- D-17: `periphore-net` is a 2-line stub; Phase 6 is its primary implementation
- D-18: Framing: `LengthDelimitedCodec` (4-byte big-endian) + `postcard` — LOCKED (Phase 1 D-13)
- D-19: `TCP_NODELAY` IMMEDIATELY after `connect()`/`accept()` — HARD REQUIREMENT (CLAUDE.md)
- D-20: `periphore-net` deps: periphore-protocol, periphore-identity, periphore-trust, periphore-config
- D-21: `periphored` adds periphore-net + periphore-core as deps in Phase 6

### Claude's Discretion
- Default TCP port value (7700–8000 range, avoid 24800; research recommends a specific port)
- Internal pending connection representation (HashMap<PeerId, PendingPeer> or similar)
- Exact exponential backoff implementation (tokio-retry crate vs manual loop)
- Whether periphore-net exposes a ConnectionManager struct or flat async API
- NetError thiserror design
- Exact wiring of FocusStateMachine into periphored (Phase 4 deferred; Phase 6 adds dep and routes SimulateEdgeCross)

### Deferred Ideas (OUT OF SCOPE)
- `periphore connect <host>` CLI command for ad-hoc connections not in config
- `periphore peers list` / `periphore peers pending` CLI subcommands
- Hot-reload of peer list without restart (hot-add of new peers deferred)
- Mutual TLS / transport-layer encryption (post-v1)
- Connection rate limiting / DDoS protection (post-v1)

</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| NET-01 | Two machines establish a peer connection over TCP | LengthDelimitedCodec + postcard framing; TcpListener accept loop; TcpStream::connect; handshake state machine |
| NET-03 | Manual host definition works as alternative to auto-discovery | PeerConfig.host/port in config schema; auto-connect on startup reads [[peer]] entries |
| NET-04 | Connections are SSH-tunnelable (TCP-only transport, no UDP) | TCP-only protocol; no UDP dependency; SSH -L port forwarding works transparently |
| NET-05 | On Linux with X-Auth, service can be launched and supervised remotely via SSH | Foreground daemon (no daemonize); systemd user unit; nohup as fallback |
| NET-06 | On other systems (macOS), daemon must be pre-running; listens on IPC + TCP | std::io::IsTerminal check on stdin at startup; clear error message + exit |

</phase_requirements>

---

## Summary

Phase 6 implements `periphore-net` from a 2-line stub into the full TCP peering layer. The primary work is: (1) a `Framed<TcpStream, LengthDelimitedCodec>` + `postcard` codec for `PeerMessage` serialization; (2) an async accept loop and per-peer outbound connection tasks with exponential backoff retry; (3) a two-phase handshake state machine (`Handshaking → Pending/Connected`) respecting the trust store; (4) a `ConnectionManager` struct that bridges `periphored`'s select! loop with the network layer via `tokio::sync::mpsc`; (5) the `daemon.listen` config field; (6) macOS SSH detection using `std::io::IsTerminal` (no libc crate needed — `libc` is a transitive dep via tokio but `std::io::IsTerminal` is available since Rust 1.70, and this project uses Rust 1.95); and (7) a `contrib/periphored.service` systemd user unit.

The framing decision is locked: `LengthDelimitedCodec::new()` (default = 4-byte big-endian u32 length header) + `postcard::to_allocvec` / `postcard::from_bytes`. No alternative framing should be explored. `TCP_NODELAY` is a hard requirement set immediately after every `connect()`/`accept()`.

**Primary recommendation:** Use a `ConnectionManager` struct with internal `tokio::task::JoinSet` for connection tasks and `tokio::sync::mpsc` channels for bidirectional communication with `periphored`. Implement backoff with a manual `tokio::time::sleep` loop using doubling delays capped at 30s — this avoids adding a new dependency for a pattern that is 10 lines of code. Default TCP port: **7888** (IANA unassigned, 7888–7899 range is explicitly unassigned in the IANA registry).

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| TCP bind / accept loop | periphore-net | — | Network I/O belongs in the net crate |
| TCP connect + retry | periphore-net | — | Connection lifecycle is net's responsibility |
| Protocol framing (LengthDelimitedCodec + postcard) | periphore-net | — | Transport encoding is net crate's internal detail |
| Handshake state machine | periphore-net | — | Handshake is a network protocol concern |
| Trust check during handshake | periphore-net (calls periphore-trust) | — | Net crate calls trust store APIs; trust store is external |
| Pending connection tracking | periphore-net (ConnectionManager) | periphored (IPC bridging) | Net owns the connection state; daemon bridges to IPC |
| AcceptFingerprint promotion | periphored (IPC handler) → periphore-net | — | IPC command arrives in daemon; daemon tells net to promote |
| GetPendingVerifications response | periphored (IPC handler) ← periphore-net | — | Daemon queries ConnectionManager, formats IpcResponse |
| FocusStateMachine wiring | periphored (select! loop) → periphore-core | — | Pure logic; daemon owns routing, calls state machine |
| macOS SSH detection | periphored (main.rs startup) | — | Daemon binary responsibility; cfg-gated |
| systemd unit file | contrib/ | — | Documentation artifact, not code |
| Config field daemon.listen | periphore-config (schema) | periphored (reads it) | Schema is config crate; daemon reads at startup |

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.52 (workspace) | Async TCP + task spawning | Already in workspace; required |
| tokio-util | 0.7 (workspace) | LengthDelimitedCodec, FramedRead/FramedWrite | Locked framing decision (D-18) |
| bytes | 1.11 (workspace) | BytesMut for codec buffers | Required by tokio-util codec |
| postcard | 1.1 (workspace) | PeerMessage serialization/deserialization | Locked serialization decision |
| thiserror | 2.0 (workspace) | NetError enum derivation | Established pattern in all library crates |
| tracing | 0.1 (workspace) | Runtime event logging | Established project pattern |
| serde | 1.0 (workspace) | Derive for any types needing it | Already in net crate Cargo.toml |

All deps above are already declared in the workspace and in `crates/periphore-net/Cargo.toml`. [VERIFIED: crates/periphore-net/Cargo.toml]

### Internal Crate Dependencies (to add to periphore-net Cargo.toml)
| Crate | Purpose |
|-------|---------|
| periphore-protocol | PeerMessage, IpcResponse types |
| periphore-identity | IdentityStore — reads local fingerprint for Hello message |
| periphore-trust | TrustStore — is_trusted(), check_peer_fingerprint() during handshake |
| periphore-config | PeerConfig, DaemonConfig — port, listen, peer entries |

Per D-20, these four internal deps must be added to `periphore-net/Cargo.toml`. [VERIFIED: 06-CONTEXT.md D-20]

### Dependencies to Add to periphored Cargo.toml
Per D-21: `periphore-net` and `periphore-core` must be added to `crates/periphored/Cargo.toml`. [VERIFIED: crates/periphored/Cargo.toml — neither is present]

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Manual backoff loop | tokio-retry / tokio-retry2 | tokio-retry2 v0.9 is actively maintained (last commit 2025-02), MSRV 1.88 which matches our toolchain; but the pattern is 10 lines — no dependency justified |
| std::io::IsTerminal | libc::isatty(0) | std::io::IsTerminal is stable since Rust 1.70 (current: 1.95); no libc crate needed at the periphored level |

**Version verification:** Workspace deps verified against registry on 2026-04-22 per Cargo.toml comment. [VERIFIED: Cargo.toml]

---

## Architecture Patterns

### System Architecture Diagram

```
                    ┌─────────────────────────────────────────┐
                    │             periphored (daemon)          │
                    │                                          │
  IPC clients ──── │ IpcServer ──mpsc──> select! loop         │
  (periphore CLI)  │                         │                 │
                    │                    IpcCommand::          │
                    │                 GetPendingVerifications  │
                    │                 AcceptFingerprint        │
                    │                         │                │
                    │              ┌──────────▼──────────┐    │
                    │              │  ConnectionManager   │    │
                    │              │  (periphore-net)     │    │
                    │              │                      │    │
                    │              │  HashMap<PeerId,     │    │
                    │              │    PendingPeer>      │    │
                    │              │  HashMap<PeerId,     │    │
                    │              │    ActiveConn>       │    │
                    │              │                      │    │
                    │              │  JoinSet (tasks):    │    │
                    │              │  - accept_loop       │    │
                    │              │  - per-peer sender   │    │
                    │              │  - per-peer receiver │    │
                    │              │  - retry connector   │    │
                    │              └───────┬──────────────┘    │
                    │                      │                   │
                    │              mpsc PeerEvent channel       │
                    │              (net→daemon)                 │
                    └─────────────────────────────────────────┘
                                           │
                         ┌─────────────────┼───────────────────┐
                         ▼                 ▼                   ▼
                   TcpListener        TcpStream            TcpStream
                   (accept loop)    (outbound peer A)   (inbound peer B)
                         │                 │                   │
                    LengthDelimited   LengthDelimited     LengthDelimited
                    Codec+postcard    Codec+postcard      Codec+postcard
                         │                 │                   │
                   PeerMessage        PeerMessage         PeerMessage
                   Hello/HelloAck     Hello/HelloAck      Hello/HelloAck
```

Data flows: network bytes → LengthDelimitedCodec framing → postcard deserialization → PeerMessage → handshake logic → PeerEvent → periphored select! loop → IPC responses.

### Recommended Module Structure for periphore-net

```
crates/periphore-net/
├── Cargo.toml
├── src/
│   ├── lib.rs          # pub use, crate-level docs
│   ├── error.rs        # NetError (thiserror)
│   ├── codec.rs        # PeerCodec: encode/decode PeerMessage via postcard+LengthDelimitedCodec
│   ├── handshake.rs    # perform_handshake() — drives Hello/HelloAck exchange
│   ├── connection.rs   # PendingPeer, ActiveConn, ConnectionState types
│   ├── manager.rs      # ConnectionManager struct, spawn_listener, spawn_connector
│   └── event.rs        # PeerEvent enum (PeerConnected, PeerPending, PeerDisconnected, etc.)
└── tests/
    └── integration.rs  # Two in-process TcpListeners, full handshake, pending/promote tests
```

### Pattern 1: Codec — Framing PeerMessage with LengthDelimitedCodec + postcard

The codec uses `LengthDelimitedCodec::new()` which defaults to 4-byte big-endian length header (matching the locked D-18 spec). `FramedRead` and `FramedWrite` are created from `TcpStream::into_split()` halves so read and write tasks can run concurrently.

```rust
// Source: Context7 /websites/rs_tokio-util — LengthDelimitedCodec docs
// Source: Context7 /websites/rs_postcard_postcard — postcard::to_allocvec, postcard::from_bytes
use bytes::{Bytes, BytesMut};
use postcard::{from_bytes, to_allocvec};
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use periphore_protocol::PeerMessage;
use crate::error::NetError;

/// Split a TcpStream into typed read/write halves.
/// TCP_NODELAY MUST be set before calling this (CLAUDE.md hard requirement).
pub fn split_framed(
    stream: TcpStream,
) -> (
    FramedRead<tokio::net::tcp::OwnedReadHalf, LengthDelimitedCodec>,
    FramedWrite<tokio::net::tcp::OwnedWriteHalf, LengthDelimitedCodec>,
) {
    // LengthDelimitedCodec::new() = 4-byte big-endian u32 length header (D-18)
    let codec = LengthDelimitedCodec::new();
    let (read_half, write_half) = stream.into_split();
    let framed_read = FramedRead::new(read_half, codec.clone());
    let framed_write = FramedWrite::new(write_half, LengthDelimitedCodec::new());
    (framed_read, framed_write)
}

/// Encode a PeerMessage to Bytes for sending via FramedWrite.
pub fn encode_message(msg: &PeerMessage) -> Result<Bytes, NetError> {
    let bytes = to_allocvec(msg).map_err(|e| NetError::Encode(e.to_string()))?;
    Ok(Bytes::from(bytes))
}

/// Decode a PeerMessage from a BytesMut frame received via FramedRead.
pub fn decode_message(frame: BytesMut) -> Result<PeerMessage, NetError> {
    from_bytes(&frame).map_err(|e| NetError::Decode(e.to_string()))
}
```

Note: `LengthDelimitedCodec` does not implement `Clone` in all versions. Create two separate `LengthDelimitedCodec::new()` instances — one for the reader, one for the writer. [ASSUMED — verify at compile time]

### Pattern 2: TCP_NODELAY Immediately After connect/accept

```rust
// Source: Context7 /websites/rs_tokio — TcpStream.set_nodelay docs
// Source: CLAUDE.md "TCP_NODELAY must be set immediately"
use tokio::net::{TcpListener, TcpStream};

// Outbound connection
async fn connect_to_peer(addr: &str) -> Result<TcpStream, NetError> {
    let stream = TcpStream::connect(addr).await.map_err(NetError::Io)?;
    // HARD REQUIREMENT: set immediately after connect (CLAUDE.md item 1)
    stream.set_nodelay(true).map_err(NetError::Io)?;
    Ok(stream)
}

// Inbound accept loop
async fn accept_loop(listener: TcpListener, /* ... */) {
    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                // HARD REQUIREMENT: set immediately after accept (CLAUDE.md item 1)
                if let Err(e) = stream.set_nodelay(true) {
                    tracing::error!(%peer_addr, error = %e, "failed to set TCP_NODELAY — dropping");
                    continue;
                }
                // spawn handshake task
                tokio::spawn(handle_inbound(stream, peer_addr, /* channels */));
            }
            Err(e) => {
                tracing::error!(error = %e, "accept error");
                // brief sleep to avoid spinning on persistent accept errors
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
}
```

### Pattern 3: Handshake State Machine

The handshake is a sequential async function (not a poll-based state machine) because it runs in its own spawned task. The states are implicit in the control flow.

```rust
// Handshake outcome returned to ConnectionManager
pub enum HandshakeResult {
    Trusted { peer_id: PeerId, fingerprint: [u8; 32] },
    Pending { peer_id: PeerId, fingerprint: [u8; 32], identicon: String, word_phrase: Vec<String> },
    Rejected { reason: String },
}

/// Perform the Hello/HelloAck handshake as the initiating side.
/// Returns HandshakeResult indicating whether the peer is trusted, pending, or rejected.
async fn perform_handshake_initiator(
    framed_read: &mut FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
    framed_write: &mut FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
    local_identity: &IdentityStore,
    trust_store: &TrustStore,
    peer_config: Option<&PeerConfig>,
) -> Result<HandshakeResult, NetError> {
    use futures::SinkExt as _;
    use tokio_stream::StreamExt as _;

    // Step 1: Send Hello with our identity
    let hello = PeerMessage::Hello {
        protocol_version: PROTOCOL_VERSION, // 1u32
        fingerprint: local_identity.fingerprint_bytes(),
        public_key: local_identity.public_key_bytes(),
    };
    framed_write.send(encode_message(&hello)?).await.map_err(NetError::Io)?;

    // Step 2: Receive peer's HelloAck (containing their identity)
    let frame = framed_read.next().await
        .ok_or(NetError::ConnectionClosed)?
        .map_err(NetError::Io)?;
    let peer_hello_ack = decode_message(frame)?;

    let (peer_fp, peer_pubkey, accepted) = match peer_hello_ack {
        PeerMessage::HelloAck { fingerprint, public_key, accepted } => {
            (fingerprint, public_key, accepted)
        }
        other => return Err(NetError::UnexpectedMessage(format!("{other:?}"))),
    };

    if !accepted {
        return Ok(HandshakeResult::Rejected { reason: "peer rejected our identity".into() });
    }

    // Step 3: Protocol version compatibility already checked by peer via their HelloAck.accepted
    // Check hard-config fingerprint constraint (SEC-06, Phase 3 D-14)
    let peer_fp_hex = hex::encode(peer_fp);
    if let Some(cfg) = peer_config {
        if let Some(configured_fp) = &cfg.fingerprint {
            periphore_trust::check_peer_fingerprint(configured_fp, &peer_fp_hex, "peer")
                .map_err(|e| NetError::FingerprintConflict(e.to_string()))?;
        }
    }

    // Step 4: Respond with our HelloAck
    let ack = PeerMessage::HelloAck {
        fingerprint: local_identity.fingerprint_bytes(),
        public_key: local_identity.public_key_bytes(),
        accepted: true,
    };
    framed_write.send(encode_message(&ack)?).await.map_err(NetError::Io)?;

    // Step 5: Check trust store
    let peer_id = PeerId::new(peer_fp_hex.clone());
    if trust_store.is_trusted(&peer_fp_hex) {
        Ok(HandshakeResult::Trusted { peer_id, fingerprint: peer_fp })
    } else {
        // Unknown peer — surface for user verification
        tracing::warn!(
            fingerprint = %peer_fp_hex,
            "unknown peer — holding in pending state. Run: periphore trust accept {}",
            &peer_fp_hex[..16]
        );
        Ok(HandshakeResult::Pending {
            peer_id,
            fingerprint: peer_fp,
            identicon: /* call resolve_identicon */ String::new(),
            word_phrase: vec![],
        })
    }
}
```

Note: The responder side mirrors this flow but receives Hello first, then sends HelloAck. Implement as `perform_handshake_responder` with symmetric logic.

### Pattern 4: Exponential Backoff — Manual Loop (Recommended)

Do NOT add tokio-retry as a dependency. The pattern is simple and self-documenting:

```rust
// Recommended: manual loop, no new dep
// Backoff schedule: 1s → 2s → 4s → 8s → 16s → cap 30s (D-06, D-09)
const BACKOFF_INITIAL_MS: u64 = 1_000;
const BACKOFF_CAP_MS: u64 = 30_000;

pub async fn connect_with_retry(
    peer_config: PeerConfig,
    event_tx: mpsc::Sender<PeerEvent>,
    shutdown: tokio_util::sync::CancellationToken,
) {
    let mut delay_ms = BACKOFF_INITIAL_MS;
    loop {
        let host = match &peer_config.host {
            Some(h) => h.clone(),
            None => return, // no host configured; nothing to connect to
        };
        let port = peer_config.port.unwrap_or(DEFAULT_PORT);
        let addr = format!("{host}:{port}");

        tracing::info!(addr = %addr, delay_ms, "attempting peer connection");

        match connect_to_peer(&addr).await {
            Ok(stream) => {
                // Reset backoff on successful connect
                delay_ms = BACKOFF_INITIAL_MS;
                // Run handshake + connection loop; returns when connection drops
                let _ = run_connection(stream, peer_config.clone(), event_tx.clone()).await;
            }
            Err(e) => {
                tracing::info!(addr = %addr, error = %e, delay_ms, "peer connection failed — retrying");
            }
        }

        // Check for shutdown before sleeping
        tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::debug!("retry loop cancelled for {addr}");
                return;
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(delay_ms)) => {}
        }

        // Double delay, cap at 30s
        delay_ms = (delay_ms * 2).min(BACKOFF_CAP_MS);
    }
}
```

### Pattern 5: ConnectionManager Struct Design

Use a struct with `spawn_*` methods. Flat async functions would not provide a coherent place to track pending connections or expose control operations (AcceptFingerprint promotion).

```rust
// periphore-net/src/manager.rs
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

pub struct ConnectionManager {
    /// Channel for net→daemon events (PeerConnected, PeerPending, PeerDisconnected, etc.)
    event_tx: mpsc::Sender<PeerEvent>,
    /// Per-peer cancellation tokens — cancel to kill a connection/retry task
    peer_tokens: HashMap<String, CancellationToken>,  // keyed by config-provided name/host
    /// Pending connections awaiting user acceptance (fingerprint_hex → PendingPeer)
    pending: HashMap<String, PendingPeer>,
    /// Active (trusted) connections (fingerprint_hex → connection handle)
    active: HashMap<String, ActiveConn>,
}

pub struct PendingPeer {
    pub fingerprint_hex: String,
    pub identicon: String,
    pub word_phrase: Vec<String>,
    /// Channel to send PromoteTrusted command to the peer's connection task
    pub promote_tx: mpsc::Sender<ConnectionControl>,
}

pub enum ConnectionControl {
    /// User accepted the fingerprint — promote to active
    PromoteTrusted,
    /// User rejected — close connection
    Reject,
}

impl ConnectionManager {
    pub fn new(event_tx: mpsc::Sender<PeerEvent>) -> Self { /* ... */ }

    /// Start the TCP listener task. Returns immediately; task runs in background.
    pub fn spawn_listener(
        &mut self,
        tasks: &mut JoinSet<anyhow::Result<()>>,
        bind_addr: std::net::SocketAddr,
        identity: Arc<IdentityStore>,
        trust_store: Arc<TrustStore>,
    ) { /* ... */ }

    /// Start an outbound connector task for a configured peer.
    pub fn spawn_connector(
        &mut self,
        tasks: &mut JoinSet<anyhow::Result<()>>,
        peer_config: PeerConfig,
        identity: Arc<IdentityStore>,
        trust_store: Arc<TrustStore>,
    ) { /* ... */ }

    /// Promote a pending connection to trusted (called by periphored on AcceptFingerprint IPC).
    pub async fn promote_pending(&mut self, fingerprint_hex: &str) -> Result<(), NetError> {
        if let Some(pending) = self.pending.get(fingerprint_hex) {
            pending.promote_tx.send(ConnectionControl::PromoteTrusted).await
                .map_err(|_| NetError::PeerNotFound(fingerprint_hex.to_owned()))
        } else {
            Err(NetError::PeerNotFound(fingerprint_hex.to_owned()))
        }
    }

    /// List pending connections (for GetPendingVerifications IPC).
    pub fn pending_list(&self) -> Vec<PendingPeerInfo> {
        self.pending.values().map(|p| PendingPeerInfo {
            fingerprint: p.fingerprint_hex.clone(),
            identicon: p.identicon.clone(),
            word_phrase: p.word_phrase.clone(),
        }).collect()
    }

    /// Cancel the retry/connection task for a specific peer (D-11: peer removed from config).
    pub fn cancel_peer(&mut self, peer_key: &str) {
        if let Some(token) = self.peer_tokens.remove(peer_key) {
            token.cancel();
        }
    }
}
```

### Pattern 6: IPC Protocol Extension — GetPendingVerifications

The current `IpcResponse` lacks a variant for pending peer data. Phase 6 must add one to `periphore-protocol/src/ipc.rs`:

```rust
// Add to IpcResponse enum in periphore-protocol/src/ipc.rs
/// Response to GetPendingVerifications (Phase 6).
PendingPeers {
    peers: Vec<PendingPeerInfo>,
},

// New supporting type (add to periphore-protocol/src/types.rs or ipc.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPeerInfo {
    /// 64-char lowercase hex fingerprint.
    pub fingerprint: String,
    /// Pre-rendered Drunken Bishop identicon string (11 lines).
    pub identicon: String,
    /// 6 BIP39 word phrase for verbal verification.
    pub word_phrase: Vec<String>,
}
```

The `ListPeers` IpcResponse variant (`Peers { peers: Vec<String> }`) already exists but carries only string fingerprints. Do not repurpose it for pending peers — the data shapes are different and commingling active/pending would confuse callers.

### Pattern 7: macOS SSH Detection (std::io::IsTerminal — No libc Needed)

Rust 1.70 introduced `std::io::IsTerminal`. The project uses Rust 1.95 — no libc crate required. `libc` is available as a transitive dep via tokio but should not be used directly for this. [VERIFIED: rustc --version → 1.95.0; docs.rust-lang.org/std/io/trait.IsTerminal.html]

```rust
// periphored/src/main.rs — add at the top of main(), before any async setup
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

### Pattern 8: Config Schema Evolution — daemon.listen Field

Add to `periphore-config/src/schema.rs`:

```rust
/// Daemon process configuration.
#[derive(Debug, Deserialize, Default)]
pub struct DaemonConfig {
    pub socket_path: Option<std::path::PathBuf>,
    pub port: Option<u16>,
    /// Whether the daemon listens for incoming peer TCP connections.
    /// Default: true (P2P symmetric model).
    /// Set to false for CI/testing setups that should not accept incoming peers.
    #[serde(default = "default_listen")]
    pub listen: bool,
}

fn default_listen() -> bool { true }
```

The `daemon.listen` field is restart-required (like `daemon.port` and `daemon.socket_path`). Add it to the restart-required check in `reload_config()` in `periphored/src/main.rs`.

### Pattern 9: periphored select! Integration

Phase 6 adds a `net_event_rx` channel to the existing select! loop:

```rust
// In periphored/src/main.rs — new channel + manager initialization
let (net_event_tx, mut net_event_rx) = mpsc::channel::<PeerEvent>(64);
let mut conn_mgr = periphore_net::ConnectionManager::new(net_event_tx);
let mut focus_sm = periphore_core::FocusStateMachine::new();

// Spawn listener if daemon.listen is true
if config.daemon.listen {
    let port = config.daemon.port.unwrap_or(DEFAULT_PORT);
    let bind_addr = format!("0.0.0.0:{port}").parse()?;
    conn_mgr.spawn_listener(&mut tasks, bind_addr, Arc::clone(&identity), Arc::clone(&trust_store));
}

// Spawn outbound connectors for all configured peers with host set
for peer in &config.peers {
    if peer.host.is_some() {
        conn_mgr.spawn_connector(&mut tasks, peer.clone(), Arc::clone(&identity), Arc::clone(&trust_store));
    }
}

// In the select! loop, add:
net_event = net_event_rx.recv() => {
    match net_event {
        Some(PeerEvent::PeerPending { fingerprint, identicon, word_phrase }) => {
            tracing::warn!(
                fingerprint = %fingerprint,
                "unknown peer — pending verification. identicon:\n{identicon}\nphrase: {}",
                word_phrase.join(" ")
            );
        }
        Some(PeerEvent::PeerConnected { peer_id }) => {
            tracing::info!(peer_id = %peer_id, "peer connected and trusted");
        }
        Some(PeerEvent::PeerDisconnected { peer_id }) => {
            tracing::info!(peer_id = %peer_id, "peer disconnected");
            let _ = focus_sm.reclaim(); // Return focus if we were forwarding to this peer
        }
        None => { /* channel closed */ }
    }
}

// AcceptFingerprint must also promote the pending connection:
Some(IpcCommand::AcceptFingerprint { fingerprint, responder }) => {
    match trust_store.add_trusted(&fingerprint, None, &trust_path) {
        Ok(()) => {
            // Phase 6: also promote pending connection
            let _ = conn_mgr.promote_pending(&fingerprint).await;
            let _ = responder.send(IpcResponse::Ok);
        }
        // ...
    }
}

// GetPendingVerifications is now real:
Some(IpcCommand::GetPendingVerifications { responder }) => {
    let peers = conn_mgr.pending_list();
    let _ = responder.send(IpcResponse::PendingPeers { peers });
}
```

### Pattern 10: SimulateEdgeCross Wiring for FocusStateMachine

Phase 4 deferred this. Phase 6 wires it:

```rust
Some(IpcCommand::SimulateEdgeCross { edge: _, position: _, responder }) => {
    // Phase 6: demonstrate FocusStateMachine wiring.
    // Real topology routing is Phase 8. For now, log the transition.
    tracing::debug!("IPC: SimulateEdgeCross — focus state: {:?}", focus_sm.current_state());
    // In Phase 8, this will drive an actual FocusTransfer to the mapped peer.
    let _ = responder.send(IpcResponse::Ok);
}
```

### Anti-Patterns to Avoid

- **`accept()` without TCP_NODELAY:** Setting it after any data exchange defeats the purpose — Nagle may batch the initial handshake bytes. Set it first, always.
- **Shared `Arc<Mutex<TrustStore>>` across connection tasks:** Prefer passing an `Arc<TrustStore>` where `TrustStore` has interior mutability (`Mutex` inside the store) or clone the store state at handshake time. Adding a trusted fingerprint during AcceptFingerprint goes through the daemon's single-threaded select! loop, so the trust store only needs `Arc<TrustStore>` with the store's own internal `Mutex` if concurrent reads are needed. [ASSUMED — evaluate at implementation time]
- **Unbounded retry without cancellation:** Every retry loop must check a `CancellationToken` (tokio_util::sync::CancellationToken) so removed peers can be stopped.
- **Panic on bad peer data:** Wrap per-connection logic in `tokio::spawn` with graceful error logging — a bad peer should not crash the daemon.
- **Using Framed on a non-split stream for concurrent I/O:** `Framed<TcpStream, ...>` requires `&mut self` for both reads and writes. Always call `into_split()` first, then create separate `FramedRead` and `FramedWrite` for concurrent send/receive tasks.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Frame delimiting | Custom length-prefix reader | `LengthDelimitedCodec::new()` | Handles partial reads, buffer management, max frame length guard |
| Message serialization | Custom binary encoder | `postcard::to_allocvec` / `postcard::from_bytes` | Compact, correct, already a workspace dep |
| Task lifecycle / cancellation | `Arc<AtomicBool>` shutdown flags | `tokio_util::sync::CancellationToken` | Composable, cloneable, integrates with select! |
| TTY detection | `libc::isatty(0)` | `std::io::IsTerminal` | Stable std since Rust 1.70; no extra dep |
| Async task joining | `Vec<JoinHandle>` + manual await | `tokio::task::JoinSet` | Already used in periphored; consistent pattern |

**Key insight:** The tokio-util codec machinery handles all the tricky edge cases of TCP stream framing (partial frames, buffer reuse, max-frame limits). Never read raw bytes and manually parse length headers.

---

## Port Selection

**Recommendation: Port 7888**

| Port | Status | Reason for Rejection/Selection |
|------|--------|-------------------------------|
| 24800 | Informally used by Synergy/Barrier | Explicitly forbidden by D-08 |
| 7700 | IANA registered: "EM7 Secure Communications" (2008) | Taken |
| 7777 | IANA registered: "cbt" (Core Based Trees); noted unauthorized use | Taken, conflict-prone |
| 7799 | IANA registered: "altbsdp" (Alternate BSDP Service, 2007) | Taken |
| 7800 | IANA registered: "asr" (Apple Software Restore) | Taken |
| 7802 | TCP Reserved, UDP: Juniper VNTP | Reserved |
| **7888** | **IANA explicitly unassigned (7888–7899 block)** | **SELECTED** |

Source: IANA Service Name and Transport Protocol Port Number Registry, queried 2026-04-26. [VERIFIED: iana.org/assignments/service-names-port-numbers]

Port 7888 is in the unassigned block 7888–7899, has no known application collisions, and avoids all known conflicts. Define it as `pub const DEFAULT_PORT: u16 = 7888;` in `periphore-net/src/lib.rs`.

---

## Protocol Version

Use `PROTOCOL_VERSION: u32 = 1`. Mismatch handling: if the received `Hello.protocol_version != PROTOCOL_VERSION`, send `HelloAck { accepted: false }` and close the connection immediately. Log at WARN level with both versions for operator diagnostics. [ASSUMED — no spec exists; this is the standard approach]

---

## Common Pitfalls

### Pitfall 1: TCP_NODELAY After First Write
**What goes wrong:** Developer sets `TCP_NODELAY` after sending the first `Hello` message. The Hello gets delayed by Nagle because TCP_NODELAY was not set when the first write occurred.
**Why it happens:** The `set_nodelay` call is easy to defer "until the socket is fully set up."
**How to avoid:** In `connect_to_peer()` and the accept loop, `set_nodelay(true)` is the second line after `connect()`/`accept()`, before any other operations.
**Warning signs:** Handshakes that intermittently take 40–200ms in development (Nagle's 200ms timeout manifesting).

### Pitfall 2: FramedRead/FramedWrite on Unsplit Stream
**What goes wrong:** Using `Framed<TcpStream, LengthDelimitedCodec>` for concurrent read/write. The `Framed` struct takes `&mut self` for both poll_next and send, making concurrent tasks impossible without additional locking.
**Why it happens:** `Framed::new(stream, codec)` looks simpler than splitting.
**How to avoid:** Always `stream.into_split()` → `FramedRead::new(read_half, codec)` + `FramedWrite::new(write_half, codec)` for separate reader/writer tasks.
**Warning signs:** Compiler error about `&mut` borrow conflicts on `Framed`.

### Pitfall 3: Retry Loop Without Cancellation
**What goes wrong:** A peer is removed from config, but its retry task keeps running, consuming resources and occasionally connecting to the removed peer.
**Why it happens:** Simple `loop {}` with `sleep` has no shutdown path.
**How to avoid:** Every retry loop must be driven by a `CancellationToken` in the select! inside the loop. `ConnectionManager::cancel_peer()` cancels the token.
**Warning signs:** periphored process accumulates tasks over time; removed peers still appear in logs.

### Pitfall 4: Pending Promotion Race
**What goes wrong:** User runs `periphore trust accept <fp>` while the connection task is in the middle of reconnecting. The `promote_pending()` call finds no entry in `pending` (the old connection dropped, new one is in handshake).
**Why it happens:** Trust store add succeeds, but the pending entry was cleared before promotion.
**How to avoid:** After `trust_store.add_trusted()` succeeds, the next successful handshake with that peer will find it in the trust store and automatically move to `Connected`. The `promote_pending()` is an optimization for connections already sitting in pending state, not a required step. Log at INFO level if the peer is not currently in pending (not an error).
**Warning signs:** `periphore trust accept` succeeds but peer still shows as pending after reconnect.

### Pitfall 5: SIGHUP Config Reload and daemon.listen
**What goes wrong:** SIGHUP triggers config reload; `daemon.listen` changes from false to true. The accept loop was never started at startup (because listen=false), and the reload tries to start it but lacks the mechanism.
**Why it happens:** The daemon's config reload is designed for hot-reload of safe fields only.
**How to avoid:** `daemon.listen` and `daemon.port` are restart-required fields (like `daemon.socket_path`). Add them to the restart-required check in `reload_config()`. Log a warning, do not attempt to start/stop the listener on reload.
**Warning signs:** User changes listen=true in config, SIGHUPs, and the daemon log shows "config reloaded" but no TCP listener appears.

### Pitfall 6: macOS Secure Input + isatty Check Order
**What goes wrong:** The SSH detection check (`!stdin().is_terminal()`) triggers in a CI environment running the daemon headlessly on macOS where no SSH tunnel is involved.
**Why it happens:** CI runners often have no TTY attached (stdin redirected from /dev/null).
**How to avoid:** Document clearly: macOS headless use requires `launchd` (not nohup, not direct invocation from CI). The error message mentions "launchd" as the correct mechanism. This is intentional by D-15: macOS headless launch is only supported through launchd, not ad-hoc.
**Warning signs:** CI build step that runs `periphored` as a subprocess on macOS fails with the SSH error.

---

## Code Examples

### Full Codec Round-Trip Verification Pattern
```rust
// Source: Context7 /websites/rs_postcard_postcard; Context7 /websites/rs_tokio-util
#[cfg(test)]
mod tests {
    use super::*;
    use periphore_protocol::PeerMessage;

    #[test]
    fn codec_roundtrip_hello() {
        let msg = PeerMessage::Hello {
            protocol_version: 1,
            fingerprint: [0u8; 32],
            public_key: vec![1, 2, 3, 4],
        };
        let encoded = encode_message(&msg).expect("encode");
        let decoded: PeerMessage = postcard::from_bytes(&encoded).expect("decode");
        assert_eq!(msg, decoded);
    }
}
```

### Connection State PeerId Alignment with FocusStateMachine

`periphore-core::PeerId` is `PeerId(String)` where the string is the fingerprint hex. In `periphore-net`, use the same type (import from `periphore-core`) to ensure the FocusStateMachine can be driven directly with the peer ID extracted from handshake:

```rust
// After successful handshake:
let peer_id = periphore_core::PeerId::new(peer_fp_hex);
// PeerEvent::PeerConnected carries this; periphored can pass to focus_sm.transfer_to(peer_id)
```

### systemd User Unit (contrib/periphored.service)

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
# Log to journal; redirect if systemd-journald is unavailable
StandardOutput=journal
StandardError=journal
# Prevent privilege escalation
NoNewPrivileges=true

[Install]
WantedBy=default.target
```

Notes on this unit:
- `Type=simple` is correct for a foreground daemon (D-12)
- `%h` expands to the user's home directory — works for user units
- `WantedBy=default.target` is the standard for systemd user units
- `Restart=on-failure` + `RestartSec=5s` provides supervision at the OS level
- Operator installs to `~/.config/systemd/user/periphored.service`, then runs `systemctl --user enable --now periphored`
- For boot-time launch (not just login-time): `loginctl enable-linger <username>`

[CITED: systemd.io/MAN/systemd.service; ASSUMED for exact unit structure — verify against local systemd version]

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `LengthDelimitedCodec` does not implement `Clone` — two instances required for split framed | Pattern 1 | Compile-time error; trivially fixed by creating two LengthDelimitedCodec::new() instances |
| A2 | `tokio_util::sync::CancellationToken` is available in tokio-util 0.7 | Pattern 4 | If absent, use `tokio::sync::broadcast` or `mpsc` for shutdown signal instead |
| A3 | `periphore-core::PeerId` should be re-exported or imported from periphore-net | Pattern 5 / Wiring | If circular deps arise, define a local PeerId newtype in periphore-net; align with core's type at the periphored boundary |
| A4 | `Arc<TrustStore>` with internal `Mutex` is the right sharing pattern for trust store across connection tasks | Pattern 5 | Could use `clone at handshake time` (simpler, slightly stale trust data possible during SIGHUP reload) |
| A5 | Protocol version `1u32` for the Phase 6 implementation | Protocol Version | Cosmetic; any value works as long as both sides use the same constant |
| A6 | `hex` crate is needed for fingerprint hex encoding in periphore-net | Pattern 3 | Check if periphore-identity already exposes `fingerprint_hex()` and `fingerprint_bytes()` (it does: `fingerprint_hex()` is verified; `fingerprint_bytes()` needs verification) |
| A7 | IANA 7888–7899 block is unassigned as of 2026-04-26 | Port Selection | If an OS or application has claimed 7888 on target machines, configure a different port via `daemon.port` in TOML |

---

## Open Questions

1. **`IdentityStore::fingerprint_bytes()` vs `fingerprint_hex()`**
   - What we know: `fingerprint_hex()` is confirmed in `periphore-identity/src/lib.rs`. The Hello message requires `fingerprint: [u8; 32]` (raw bytes).
   - What's unclear: Does `IdentityStore` expose a method to get raw `[u8; 32]`? Or must we decode the hex string?
   - Recommendation: Add `pub fn fingerprint_bytes(&self) -> [u8; 32]` to `IdentityStore` in Phase 6 if it doesn't exist, rather than decoding hex in periphore-net.

2. **Trust store thread safety**
   - What we know: `TrustStore` has no `Send + Sync` bounds; its `add_trusted` takes `&mut self`.
   - What's unclear: Connection tasks need to call `is_trusted()` (read-only) during handshake. The daemon's select! loop calls `add_trusted()`.
   - Recommendation: Pass `Arc<RwLock<TrustStore>>` to connection tasks. The handshake task holds a read lock briefly; the daemon holds a write lock during AcceptFingerprint. This is safe and low-contention.

3. **futures crate for SinkExt**
   - What we know: `FramedWrite::send()` requires `SinkExt` from the `futures` crate (or `futures-util`).
   - What's unclear: Is `futures` already a transitive dep of tokio-util that re-exports the trait?
   - Recommendation: Add `futures-util` to `periphore-net/Cargo.toml` if `SinkExt` and `StreamExt` are needed. Alternatively, use `FramedWrite::get_mut().write_all()` pattern but that bypasses codec framing — use the `SinkExt` path.

---

## Environment Availability

Step 2.6: No new external tools required. All external dependencies are Rust crates already in the workspace or easily added. No databases, no external services, no CLI tools beyond `cargo`/`rustc`.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Build | ✓ | 1.95.0 | — |
| tokio (workspace) | Async runtime | ✓ | 1.52 | — |
| tokio-util (workspace) | LengthDelimitedCodec | ✓ | 0.7 | — |
| postcard (workspace) | Serialization | ✓ | 1.1 | — |
| std::io::IsTerminal | macOS TTY check | ✓ | Stable since Rust 1.70 | — |
| libc (transitive) | — | ✓ | 0.2.185 (via tokio) | Not needed |
| systemd (Linux) | NET-05 supervision | ✓ (Linux targets) | — | nohup fallback documented |

**Missing dependencies with no fallback:** None.

---

## Validation Architecture

nyquist_validation is enabled in `.planning/config.json`. [VERIFIED: .planning/config.json]

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `#[tokio::test]` |
| Config file | None — `[lib] test = false` + integration tests in `tests/` per workspace pattern |
| Quick run command | `cargo test -p periphore-net` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| NET-01 | Two peers complete Hello/HelloAck handshake in-process | integration | `cargo test -p periphore-net --test integration` | ❌ Wave 0 |
| NET-01 | Trusted peer transitions to Connected state | integration | `cargo test -p periphore-net --test integration trust_handshake` | ❌ Wave 0 |
| NET-01 | Unknown peer transitions to Pending state | integration | `cargo test -p periphore-net --test integration pending_handshake` | ❌ Wave 0 |
| NET-03 | PeerConfig.host triggers auto-connect on startup | integration | `cargo test -p periphored --test net_wiring` | ❌ Wave 0 |
| NET-04 | TCP-only framing: no UDP calls in periphore-net | static (grep) | No UDP imports in periphore-net | — |
| NET-05 | Daemon stays running after SSH session (nohup/systemd) | manual | — | manual-only |
| NET-06 | macOS SSH detection exits with clear error | unit | `cargo test -p periphored --test macos_ssh` | ❌ Wave 0 |
| NET-06 | macOS SSH check is cfg-gated (Linux ignores it) | unit | `cargo test -p periphored` (Linux CI) | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p periphore-net -p periphored`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/periphore-net/tests/integration.rs` — covers NET-01 (handshake), NET-01 (pending), NET-01 (trust)
- [ ] `crates/periphored/tests/net_wiring.rs` — covers NET-03 (auto-connect from config)
- [ ] macOS SSH detection: `#[cfg(target_os = "macos")]` unit test in `periphored/src/main.rs` or a test module

**Integration test design for periphore-net:** Bind a `TcpListener` on `127.0.0.1:0` (OS assigns port), connect from a second task, run the handshake with a fabricated `IdentityStore` and an empty `TrustStore`. Assert `HandshakeResult::Pending` for unknown peer. Then call `promote_pending()` and assert `PeerEvent::PeerConnected`. This is fully in-process and requires no external infrastructure.

---

## Security Domain

security_enforcement is enabled (Level 1) per `.planning/config.json`.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | Yes — peer identity via Ed25519 fingerprint | periphore-identity fingerprint; handshake Hello/HelloAck |
| V3 Session Management | Yes — pending vs connected state; trust promotion | ConnectionManager PendingPeer → ActiveConn state machine |
| V4 Access Control | Yes — untrusted peers must not receive input events | Input forwarding blocked until user accepts; Pending state enforces this |
| V5 Input Validation | Yes — PeerMessage deserialization | postcard::from_bytes returns Err on malformed data; always handle Err, never unwrap |
| V6 Cryptography | Partial — fingerprint is SHA-256 of public key; no encryption at transport layer (TLS deferred) | ed25519-dalek for key operations; SHA-256 for fingerprints |

### Known Threat Patterns for Async TCP Server

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed frame (oversized length header) | Tampering | `LengthDelimitedCodec::builder().max_frame_length(64 * 1024).new_codec()` — reject frames > 64KB |
| Unknown peer sends events before acceptance | Elevation of Privilege | Pending state blocks all non-handshake messages; drop any non-Hello message in Handshaking state |
| Fingerprint replay (attacker presents stolen fingerprint) | Spoofing | Ed25519 signature verification — public key in Hello must match fingerprint; verify at handshake (Phase 6 stores public key, Phase 9 uses for message auth) |
| Accept storm / connection flood | DoS | No connection rate limiting in v1 (post-v1); mitigated by socket listen backlog; log at WARN |
| SSH tunnel spoofing | Tampering | SSH tunnel is a network path concern; Ed25519 fingerprint verification provides endpoint authentication regardless of path |

**Max frame length recommendation:** Set `max_frame_length` on the codec to prevent OOM from a malicious peer sending a 4GB frame length value:

```rust
// In codec.rs or wherever the codec is constructed
let codec = LengthDelimitedCodec::builder()
    .max_frame_length(64 * 1024) // 64 KB max frame — no PeerMessage should be larger
    .new_codec();
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `atty` crate for TTY detection | `std::io::IsTerminal` | Rust 1.70 (2023) | `atty` is unmaintained; std is stable and dependency-free |
| `tokio-retry` (original, unmaintained) | Manual loop or `tokio-retry2` fork | 2024 | Original `tokio-retry` last update was years ago; fork is active but adds no value vs 10-line manual pattern |
| `async_trait` for async trait methods | Native async fn in trait | Rust 1.75 (2023) | No `async_trait` crate needed for simple async traits |

**Deprecated/outdated:**
- `atty` crate: replaced by `std::io::IsTerminal` since Rust 1.70
- `tokio-retry` original (not `tokio-retry2`): last release ~2021; MSRV conflicts

---

## Sources

### Primary (HIGH confidence)
- Context7 `/websites/rs_tokio-util` — LengthDelimitedCodec::new(), builder(), FramedRead, FramedWrite
- Context7 `/websites/rs_tokio` — TcpStream.into_split(), set_nodelay(), TcpListener.accept()
- Context7 `/websites/rs_postcard_postcard` — to_allocvec(), from_bytes()
- docs.rust-lang.org/std/io/trait.IsTerminal.html — IsTerminal, Rust 1.70+
- IANA Service Name and Transport Protocol Port Number Registry (queried 2026-04-26) — port 7888 unassigned
- crates/periphore-net/Cargo.toml — current deps (periphore-protocol, tokio, tokio-util, bytes, serde, thiserror, tracing)
- crates/periphored/src/main.rs — existing select! loop, IpcCommand dispatch, JoinSet pattern
- crates/periphore-protocol/src/peer.rs — PeerMessage enum (all variants)
- crates/periphore-protocol/src/ipc.rs — IpcResponse variants (no PendingPeers yet)
- crates/periphore-core/src/lib.rs — FocusStateMachine, PeerId types

### Secondary (MEDIUM confidence)
- IANA registry individual port queries (7700, 7777, 7799, 7800, 7802, 7888) — cross-verified with multiple IANA registry queries
- github.com/naomijub/tokio-retry — version 0.9, last commit 2025-02, MSRV 1.88 (WebFetch verified)
- systemd.io documentation patterns — systemd user unit structure, Type=simple, WantedBy=default.target

### Tertiary (LOW confidence — flag for validation)
- LengthDelimitedCodec Clone behavior (A1 in Assumptions Log) — assumed; verify at compile time
- futures-util / SinkExt availability (A3 in Open Questions) — check crate deps before coding

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all deps verified in workspace Cargo.toml
- LengthDelimitedCodec API: HIGH — verified via Context7 tokio-util docs
- Postcard API: HIGH — verified via Context7 postcard docs
- Port 7888: HIGH — verified against IANA registry
- IsTerminal approach: HIGH — verified against official Rust docs
- Architecture patterns: HIGH — derived from existing codebase analysis and locked decisions
- ConnectionManager design: MEDIUM — design recommendation; implementation details confirmed at compile time
- systemd unit file: MEDIUM — standard structure; verify against target system's systemd version

**Research date:** 2026-04-26
**Valid until:** 2026-05-26 (30 days — stable ecosystem)
