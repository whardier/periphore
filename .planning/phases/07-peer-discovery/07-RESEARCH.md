# Phase 7: Peer Discovery - Research

**Researched:** 2026-04-28
**Domain:** mDNS service discovery + SSH tunnel port probing (Rust/Tokio)
**Confidence:** HIGH

## Summary

Phase 7 adds two peer discovery mechanisms to the Periphore daemon: (1) mDNS-based service discovery using the `mdns-sd` crate for local network peers, and (2) SSH tunnel port probing for peers reachable via forwarded ports on localhost. Both mechanisms produce a passive in-memory list of discovered peers that users can inspect via CLI (`periphore peers discovered`) before manually configuring connections.

The `mdns-sd` crate (v0.19.1) is the project's specified mDNS library. It uses a dedicated background thread with `flume` channels that support both sync and async consumption via `recv_async().await`, making it directly compatible with the existing `tokio::select!` event loop in `periphored`. The crate handles RFC 6762/6763 compliance including goodbye packets, known-answer suppression, and name conflict resolution. SSH port probing uses `tokio::time::timeout(Duration::from_millis(100), TcpStream::connect(addr))` to detect locally-forwarded Periphore daemons by performing a lightweight Hello/HelloAck exchange on a configurable port range (default 17880-17890).

The implementation lives in a new `periphore-discovery` crate with channel-based output (`mpsc::Sender<DiscoveryEvent>`) matching the `PeerEvent` pattern from `periphore-net`. Integration points are well-defined: `DiscoveryConfig` added to `periphore-config/schema.rs`, `GetDiscoveredPeers` IPC variant added to `periphore-protocol/ipc.rs` and `periphore-ipc/src/lib.rs`, and daemon wiring in `periphored/src/main.rs`.

**Primary recommendation:** Create `periphore-discovery` crate with `DiscoveryService` struct that spawns mDNS and SSH probe tasks, emits `DiscoveryEvent` variants through an mpsc channel, and maintains the discovered peer list internally with hybrid expiry (goodbye + 5-minute TTL GC).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** mDNS discovery logic lives in a **new `periphore-discovery` crate** at `crates/periphore-discovery`. Not inside `periphore-net` or `periphored` directly.
- **D-02:** `periphore-discovery` depends on `periphore-net` + `periphore-config`. Build order: after `periphore-net`, before `periphored`. Add `periphore-discovery` to workspace `Cargo.toml` `[workspace.dependencies]` and to `periphored`'s `[dependencies]`.
- **D-03:** mDNS discovery is **opt-in** -- disabled by default. User enables by adding `[discovery]\nenabled = true` to their TOML config.
- **D-04:** New top-level `[discovery]` section in `periphore-config` `schema.rs` as a `DiscoveryConfig` struct with fields: `enabled: bool` (default `false`), `instance_name: Option<String>`, `service_type: String` (default `_periphore._tcp.local.`).
- **D-05:** Discovered peers are **passive** -- tracked in memory but NOT auto-connected. No TCP connection is initiated on discovery.
- **D-06:** New `IpcRequest::GetDiscoveredPeers` variant in `periphore-protocol/src/ipc.rs`. Response: `IpcResponse::DiscoveredPeers(Vec<DiscoveredPeerInfo>)`. `DiscoveredPeerInfo` contains: `hostname: String`, `port: u16`, `last_seen: std::time::SystemTime` (or serializable timestamp).
- **D-07:** Discovered list uses **hybrid expiry**: remove entry immediately when mDNS goodbye event fires (primary); also expire entries via TTL garbage collection (safety net).
- **D-08:** TTL for garbage-collected entries: **5 minutes** since `last_seen`. Background task sweeps periodically; each re-announcement refreshes `last_seen`.
- **D-09:** Discovered list is capped at **64 peers**. When cap is hit: evict the entry with the oldest `last_seen` timestamp. Log at `tracing::warn!` on cap overflow eviction.
- **D-10:** `periphore peers discovered` -- new subcommand. Sends `IpcRequest::GetDiscoveredPeers`, displays result as table.
- **D-11:** `periphore peers pending` -- new subcommand. Sends `IpcRequest::GetPendingVerifications`.
- **D-12:** `periphore connect <host>` -- **deferred to a future phase**.
- **D-13:** `periphore peers list` (combined active + pending view) -- **deferred to a future phase**.

### Claude's Discretion
- Exact mDNS TXT record fields (hostname + port sufficient; protocol version optional)
- Internal struct representation of `DiscoveredPeerInfo` (serialization format for IPC)
- Channel-based API for `periphore-discovery` (use channel-based to match `PeerEvent` pattern)
- Exact sweep interval for the TTL GC task (30s sweep, 5-minute TTL threshold)
- Output table format for `periphore peers discovered` (align with `periphore status` output style)
- Error type design for `periphore-discovery` (`thiserror`-derived `DiscoveryError`)
- Whether the mDNS service instance name includes a random suffix for multi-daemon hosts
- SSH tunnel port probing design (config, port range, probe protocol, integration)

### Deferred Ideas (OUT OF SCOPE)
- `periphore connect <host>` on-demand connect command
- `periphore peers list` combined view (active + pending + discovered)
- Auto-connect on discovery (user chose passive model)
- TXT record fingerprint advertisement
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| NET-02 | Auto-discovery locates peers on the local network via mDNS | `mdns-sd` v0.19.1 crate provides mDNS browse/register; `DiscoveryService` wraps both mDNS and SSH probe mechanisms; discovered peer list exposed via `GetDiscoveredPeers` IPC |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| mDNS broadcast/browse | periphore-discovery | -- | Dedicated crate owns `mdns-sd` integration, service registration, event processing |
| SSH tunnel port probing | periphore-discovery | periphore-net (codec reuse) | Discovery crate owns probe loop; reuses `periphore-net::codec` + `periphore-net::handshake::PROTOCOL_VERSION` for Hello/HelloAck validation |
| Discovered peer list | periphore-discovery | periphored (IPC dispatch) | Discovery crate owns the in-memory list + TTL GC; daemon reads it via method call on IPC request |
| Discovery config | periphore-config | -- | `DiscoveryConfig` struct lives in config crate's `schema.rs` |
| IPC types (GetDiscoveredPeers) | periphore-protocol + periphore-ipc | periphored (dispatch) | Protocol crate defines request/response types; IPC crate adds `IpcCommand` variant; daemon dispatches |
| CLI (peers discovered/pending) | periphore-cli | -- | New `peers` subcommand group with `discovered` and `pending` handlers |
| Daemon wiring | periphored | -- | `main.rs` spawns `DiscoveryService`, adds `IpcCommand::GetDiscoveredPeers` dispatch arm |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| mdns-sd | 0.19.1 | mDNS service discovery (register + browse) | Specified in CLAUDE.md; pure Rust, RFC 6762/6763 compliant, no async runtime dependency, flume channels for async interop [VERIFIED: cargo search mdns-sd] |
| tokio | 1.52 (workspace) | Async runtime for SSH port probing + event loop | Already in workspace; provides `TcpStream::connect`, `tokio::time::timeout`, `tokio::select!` [VERIFIED: workspace Cargo.toml] |
| thiserror | 2.0 (workspace) | `DiscoveryError` enum | Project convention for library crate error types [VERIFIED: codebase pattern] |
| tracing | 0.1 (workspace) | Logging (warn on mDNS failure, info on discovery events) | Project convention [VERIFIED: codebase pattern] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| periphore-net | workspace | `DEFAULT_PORT`, `PROTOCOL_VERSION`, `codec::split_framed`, `codec::encode_message`, `codec::decode_message` | SSH probe: reuse existing framed codec + Hello/HelloAck messages for daemon identification |
| periphore-config | workspace | `DiscoveryConfig` struct definition, `Config.discovery` field | Config loading for discovery settings |
| periphore-protocol | workspace | `PeerMessage::Hello`, `PeerMessage::HelloAck`, `IpcRequest::GetDiscoveredPeers`, `DiscoveredPeerInfo` | SSH probe uses Hello/HelloAck; IPC types for discovered peer list |
| tokio-util | 0.7 (workspace) | `LengthDelimitedCodec` for SSH probe framed I/O | Reused via `periphore-net::codec::split_framed` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| mdns-sd | simple-mdns | Simpler API but less RFC-compliant; mdns-sd is specified in CLAUDE.md |
| mdns-sd | libmdns | Tighter tokio integration but less maintained; mdns-sd has active development |
| flume (transitive via mdns-sd) | -- | mdns-sd's `Receiver` uses flume internally; `recv_async()` is tokio-compatible without explicit flume dependency |

**Installation:**
```bash
# Add to workspace Cargo.toml [workspace.dependencies]:
# mdns-sd = { version = "0.19", default-features = true }
#
# Add to periphore-discovery Cargo.toml [dependencies]:
# mdns-sd = { workspace = true }
# periphore-net = { workspace = true }
# periphore-config = { workspace = true }
# periphore-protocol = { workspace = true }
# tokio = { workspace = true }
# thiserror = { workspace = true }
# tracing = { workspace = true }
```

**Version verification:**
- `mdns-sd`: 0.19.1 (latest on crates.io as of 2026-04-28) [VERIFIED: cargo search mdns-sd]
- All other deps use existing workspace versions [VERIFIED: workspace Cargo.toml]

## Architecture Patterns

### System Architecture Diagram

```
                     TOML Config
                         |
                         v
              +--------------------+
              | periphore-config   |
              | DiscoveryConfig    |
              +--------------------+
                         |
                         v
    +------------------------------------------------+
    |        periphore-discovery crate                |
    |                                                |
    |  DiscoveryService                              |
    |  +------------------------------------------+  |
    |  |                                          |  |
    |  |  mDNS Task (mdns-sd thread)              |  |
    |  |  - register(_periphore._tcp.local.)      |  |
    |  |  - browse(_periphore._tcp.local.)        |  |
    |  |  - ServiceEvent --> DiscoveryEvent        |  |
    |  |                                          |  |
    |  |  SSH Probe Task (tokio interval)         |  |
    |  |  - sweep ports 17880-17890               |  |
    |  |  - TcpStream::connect + Hello/HelloAck   |  |
    |  |  - valid response --> DiscoveryEvent      |  |
    |  |                                          |  |
    |  |  GC Task (tokio interval, 30s)           |  |
    |  |  - remove entries with last_seen > 5min  |  |
    |  |                                          |  |
    |  +------------------------------------------+  |
    |                    |                           |
    |        mpsc::Sender<DiscoveryEvent>            |
    |                    |                           |
    |  Discovered Peer List (Arc<Mutex<HashMap>>)    |
    |  - hostname -> DiscoveredPeerEntry             |
    |  - cap: 64, evict oldest on overflow           |
    +------------------------------------------------+
                         |
          +--------------+--------------+
          |                             |
          v                             v
    +-----------+              +-----------------+
    | periphored|              | periphore-cli   |
    | main.rs   |              |                 |
    | select!   |              | peers discovered|
    | loop      |              | peers pending   |
    +-----------+              +-----------------+
          |                             |
          +----------IPC----------------+
              GetDiscoveredPeers
              GetPendingVerifications
```

### Recommended Project Structure
```
crates/periphore-discovery/
├── Cargo.toml           # deps: mdns-sd, periphore-net, periphore-config, periphore-protocol, tokio, thiserror, tracing
├── src/
│   ├── lib.rs           # pub use, DiscoveryService, DiscoveryEvent
│   ├── error.rs         # DiscoveryError (thiserror)
│   ├── mdns.rs          # mDNS register + browse logic, ServiceEvent -> DiscoveryEvent
│   ├── probe.rs         # SSH tunnel port probe loop, Hello/HelloAck validation
│   └── list.rs          # DiscoveredPeerList: in-memory store, TTL GC, cap enforcement
└── tests/
    └── integration.rs   # mDNS register+browse round-trip, probe against test listener
```

### Pattern 1: Channel-Based Discovery Service
**What:** `DiscoveryService` spawns internal tasks and emits `DiscoveryEvent` through an `mpsc::Sender`, matching the `PeerEvent` pattern from `periphore-net::ConnectionManager`.
**When to use:** For all discovery output -- daemon consumes events in its `tokio::select!` loop.
**Example:**
```rust
// Source: matches periphore-net ConnectionManager pattern [VERIFIED: codebase]
pub enum DiscoveryEvent {
    /// A new peer was discovered or an existing one refreshed
    PeerDiscovered {
        hostname: String,
        port: u16,
        source: DiscoverySource,
    },
    /// A peer was removed (mDNS goodbye or TTL expired)
    PeerRemoved {
        hostname: String,
        port: u16,
    },
    /// mDNS daemon encountered a non-fatal error
    Error(String),
}

pub enum DiscoverySource {
    Mdns,
    SshProbe,
}

pub struct DiscoveryService {
    /// In-memory list of discovered peers, shared with GC task
    peers: Arc<Mutex<DiscoveredPeerList>>,
}

impl DiscoveryService {
    pub fn new() -> Self { ... }

    /// Start mDNS registration + browsing + SSH probe + GC tasks.
    /// Spawns into the provided JoinSet and CancellationToken.
    pub fn start(
        &self,
        tasks: &mut JoinSet<anyhow::Result<()>>,
        config: &DiscoveryConfig,
        event_tx: mpsc::Sender<DiscoveryEvent>,
        identity: Arc<IdentityStore>,
        trust_store: Arc<RwLock<TrustStore>>,
        cancel: CancellationToken,
    ) { ... }

    /// Return a snapshot of all currently discovered peers.
    /// Called by daemon on GetDiscoveredPeers IPC.
    pub fn discovered_list(&self) -> Vec<DiscoveredPeerInfo> { ... }
}
```

### Pattern 2: mDNS Integration with flume::Receiver in tokio
**What:** `mdns-sd::ServiceDaemon` uses a background thread and returns `flume::Receiver<ServiceEvent>`. The receiver's `recv_async()` returns a future compatible with `tokio::select!`.
**When to use:** The mDNS browse task consumes events from the flume receiver in a loop.
**Example:**
```rust
// Source: mdns-sd 0.19.1 docs [CITED: docs.rs/mdns-sd/0.19.1/mdns_sd/]
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

async fn mdns_browse_loop(
    service_type: &str,
    event_tx: mpsc::Sender<DiscoveryEvent>,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    let mdns = ServiceDaemon::new()
        .map_err(|e| anyhow::anyhow!("mDNS daemon failed to start: {e}"))?;

    let receiver = mdns.browse(service_type)
        .map_err(|e| anyhow::anyhow!("mDNS browse failed: {e}"))?;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                let _ = mdns.shutdown();
                break;
            }
            result = receiver.recv_async() => {
                match result {
                    Ok(ServiceEvent::ServiceResolved(info)) => {
                        let hostname = info.get_hostname().trim_end_matches('.').to_owned();
                        let port = info.get_port();
                        event_tx.send(DiscoveryEvent::PeerDiscovered {
                            hostname,
                            port,
                            source: DiscoverySource::Mdns,
                        }).await.ok();
                    }
                    Ok(ServiceEvent::ServiceRemoved(_ty, fullname)) => {
                        // mDNS goodbye -- trigger immediate removal
                        event_tx.send(DiscoveryEvent::PeerRemoved {
                            hostname: extract_hostname(&fullname),
                            port: 0, // port not in ServiceRemoved; list uses hostname as key
                        }).await.ok();
                    }
                    Ok(_) => {} // SearchStarted, ServiceFound, SearchStopped
                    Err(_) => break, // Channel closed -- mdns daemon shut down
                }
            }
        }
    }
    Ok(())
}
```

### Pattern 3: SSH Tunnel Port Probing
**What:** Periodically probe a range of localhost ports, attempting a lightweight Hello/HelloAck handshake to identify Periphore daemons reachable via SSH-forwarded ports.
**When to use:** When mDNS is unreliable (corporate networks, VPNs, different subnets) and users have set up SSH tunnels manually.
**Example:**
```rust
// Source: tokio TcpStream + periphore-net handshake [VERIFIED: codebase]
use std::time::Duration;
use tokio::net::TcpStream;

const PROBE_TIMEOUT: Duration = Duration::from_millis(100);
const PROBE_INTERVAL: Duration = Duration::from_secs(30);

async fn ssh_probe_loop(
    ports: Vec<u16>,
    event_tx: mpsc::Sender<DiscoveryEvent>,
    identity: Arc<IdentityStore>,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    loop {
        for &port in &ports {
            let addr = format!("127.0.0.1:{port}");
            // Fast timeout -- 100ms is enough for localhost
            match tokio::time::timeout(PROBE_TIMEOUT, TcpStream::connect(&addr)).await {
                Ok(Ok(stream)) => {
                    if let Err(e) = stream.set_nodelay(true) {
                        tracing::trace!(port, error = %e, "probe: TCP_NODELAY failed");
                        continue;
                    }
                    // Attempt Hello/HelloAck to verify it's a Periphore daemon
                    match probe_handshake(stream, &identity).await {
                        Ok(true) => {
                            event_tx.send(DiscoveryEvent::PeerDiscovered {
                                hostname: "127.0.0.1".to_owned(),
                                port,
                                source: DiscoverySource::SshProbe,
                            }).await.ok();
                        }
                        Ok(false) => {
                            tracing::trace!(port, "probe: not a Periphore daemon (version mismatch or wrong protocol)");
                        }
                        Err(e) => {
                            tracing::trace!(port, error = %e, "probe: handshake failed");
                        }
                    }
                }
                Ok(Err(_)) | Err(_) => {
                    // Port not listening or timeout -- normal for most ports
                }
            }
        }

        // Wait before next probe sweep
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(PROBE_INTERVAL) => {}
        }
    }
    Ok(())
}

/// Lightweight handshake probe: send Hello, expect HelloAck with matching PROTOCOL_VERSION.
/// Returns true if the remote is a Periphore daemon with compatible protocol.
/// Does NOT perform trust/identity verification -- this is discovery only.
async fn probe_handshake(stream: TcpStream, identity: &IdentityStore) -> anyhow::Result<bool> {
    let (mut fr, mut fw) = periphore_net::codec::split_framed(stream);

    // Send Hello
    let hello = periphore_protocol::PeerMessage::Hello {
        protocol_version: periphore_net::PROTOCOL_VERSION,
        fingerprint: identity.fingerprint,
        public_key: identity.keypair.verifying_key().to_bytes().to_vec(),
    };
    use futures_util::SinkExt as _;
    fw.send(periphore_net::codec::encode_message(&hello)?).await?;

    // Receive HelloAck with tight timeout (200ms for localhost)
    use futures_util::StreamExt as _;
    let frame = tokio::time::timeout(
        Duration::from_millis(200),
        fr.next(),
    ).await
        .map_err(|_| anyhow::anyhow!("probe timeout"))?
        .ok_or_else(|| anyhow::anyhow!("connection closed"))?
        .map_err(|e| anyhow::anyhow!("frame read: {e}"))?;

    let msg = periphore_net::codec::decode_message(frame)?;
    match msg {
        periphore_protocol::PeerMessage::HelloAck { accepted, .. } => Ok(accepted),
        _ => Ok(false), // Not the expected response -- not a Periphore daemon
    }
}
```

### Pattern 4: Discovered Peer List with TTL GC
**What:** In-memory `HashMap<String, DiscoveredPeerEntry>` with cap enforcement and periodic garbage collection.
**When to use:** Stores discovered peers from both mDNS and SSH probe sources.
**Example:**
```rust
// Source: matches D-07, D-08, D-09 from CONTEXT.md [VERIFIED: CONTEXT.md]
use std::collections::HashMap;
use std::time::{Instant, Duration};

const MAX_PEERS: usize = 64;
const TTL: Duration = Duration::from_secs(300); // 5 minutes

pub struct DiscoveredPeerEntry {
    pub hostname: String,
    pub port: u16,
    pub last_seen: Instant,
    pub source: DiscoverySource,
}

pub struct DiscoveredPeerList {
    entries: HashMap<String, DiscoveredPeerEntry>,
}

impl DiscoveredPeerList {
    pub fn upsert(&mut self, hostname: String, port: u16, source: DiscoverySource) {
        let key = format!("{hostname}:{port}");
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_seen = Instant::now();
            return;
        }
        // Cap enforcement: evict oldest if at capacity
        if self.entries.len() >= MAX_PEERS {
            if let Some(oldest_key) = self.entries.iter()
                .min_by_key(|(_, e)| e.last_seen)
                .map(|(k, _)| k.clone())
            {
                tracing::warn!(evicted = %oldest_key, "discovered peer list full (64) -- evicting oldest");
                self.entries.remove(&oldest_key);
            }
        }
        self.entries.insert(key, DiscoveredPeerEntry {
            hostname, port, last_seen: Instant::now(), source,
        });
    }

    pub fn remove(&mut self, hostname: &str, port: u16) {
        let key = format!("{hostname}:{port}");
        self.entries.remove(&key);
    }

    pub fn gc(&mut self) {
        let now = Instant::now();
        self.entries.retain(|_, e| now.duration_since(e.last_seen) < TTL);
    }

    pub fn snapshot(&self) -> Vec<DiscoveredPeerInfo> {
        self.entries.values().map(|e| DiscoveredPeerInfo {
            hostname: e.hostname.clone(),
            port: e.port,
            last_seen: /* convert Instant to SystemTime */ ...,
        }).collect()
    }
}
```

### Anti-Patterns to Avoid
- **Auto-connecting on discovery:** D-05 explicitly forbids this. Discovery is passive -- the user decides when to connect by adding `[[peer]]` config.
- **Blocking mDNS calls in async context:** `mdns-sd` uses flume channels -- always use `recv_async()`, never `recv()` in tokio tasks.
- **Failing daemon startup on mDNS bind error:** Per CLAUDE.md item 6 and specific discussion, mDNS bind failure must log `tracing::warn!` and continue. The daemon is fully functional without discovery.
- **Using `SystemTime` for internal TTL tracking:** Use `std::time::Instant` internally (monotonic, not affected by clock adjustments). Convert to `SystemTime` only for IPC serialization.
- **Probing the daemon's own port:** SSH probe must skip `DEFAULT_PORT` (7888) on localhost -- that's the local daemon itself, not a forwarded peer.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| mDNS protocol | Raw UDP multicast + DNS record parsing | `mdns-sd` crate | RFC 6762/6763 compliance requires known-answer suppression, probing, conflict resolution, goodbye packets -- hundreds of edge cases |
| Service discovery events | Custom socket polling | `mdns-sd::ServiceDaemon::browse()` | Thread-safe, channel-based, handles retransmission and caching internally |
| Framed TCP protocol | Raw TCP read/write for SSH probe | `periphore-net::codec::split_framed` + `encode_message`/`decode_message` | Already exists, handles `LengthDelimitedCodec` + postcard serialization |
| Hello/HelloAck protocol | Custom probe message | Reuse `PeerMessage::Hello`/`PeerMessage::HelloAck` | Ensures the probe validates real protocol compatibility, not just port openness |

**Key insight:** The existing `periphore-net` codec and handshake infrastructure provides exactly the right tool for SSH port probing. Sending a real Hello and expecting a real HelloAck with matching `PROTOCOL_VERSION` guarantees the remote is a compatible Periphore daemon, not a random service.

## Common Pitfalls

### Pitfall 1: mDNS Fails Silently on Corporate Networks
**What goes wrong:** `ServiceDaemon::new()` succeeds but multicast packets are blocked by the network. No peers are ever discovered, and no error is reported.
**Why it happens:** Corporate firewalls, VPN tunnels, and network policies block multicast traffic on port 5353. The mdns-sd daemon binds successfully but packets never reach peers.
**How to avoid:** (1) Log at `tracing::info!` when discovery is enabled so users can verify it's running. (2) The SSH probe mechanism provides the fallback for exactly this scenario. (3) Manual `[[peer]] host=` config always works regardless of discovery status.
**Warning signs:** `periphore peers discovered` returns empty list for extended periods while peers are running on the same network.

### Pitfall 2: ServiceRemoved Does Not Contain Full Address Info
**What goes wrong:** `ServiceEvent::ServiceRemoved(service_type, fullname)` provides the service fullname but NOT the IP/port. If the peer list is keyed by `hostname:port`, you need to parse the fullname to extract the instance name and look up the matching entry.
**Why it happens:** RFC 6762 goodbye packets contain only the service name, not the full SRV record data.
**How to avoid:** Key the internal list by a composite key derived from the mDNS fullname (e.g., `instance_name.service_type`) and maintain a reverse mapping to `hostname:port`. Or, more simply, key by `hostname:port` and store the fullname as a field for matching against `ServiceRemoved` events.
**Warning signs:** Goodbye events silently fail to remove peers from the discovered list.

### Pitfall 3: SSH Probe Connects to Own Daemon
**What goes wrong:** The probe loop discovers `127.0.0.1:7888` -- which is the local daemon's own listener. This appears as a "discovered peer" that is actually the local machine.
**Why it happens:** The daemon listens on `0.0.0.0:7888` which includes localhost. If the probe range includes 7888, it will connect to itself.
**How to avoid:** (1) Default probe range is 17880-17890, which does not include 7888. (2) Explicitly filter out the daemon's own configured port from the probe range. (3) Compare the HelloAck fingerprint against the local identity -- if it matches, skip (same daemon).
**Warning signs:** `periphore peers discovered` shows a peer with the local machine's fingerprint.

### Pitfall 4: SSH Probe Leaves Connections in Pending State
**What goes wrong:** The probe performs a full Hello/HelloAck handshake. The remote daemon's `ConnectionManager` receives this connection and emits `PeerEvent::PeerPending` because the prober's fingerprint is unknown. This creates ghost pending entries.
**Why it happens:** The probe is using the real handshake protocol, which triggers the real handshake flow on the remote side.
**How to avoid:** The probe should disconnect immediately after receiving HelloAck -- do not keep the connection open. The remote's accept loop task will see the connection close and clean up the pending entry (since `promote_rx.recv()` returns `None` when the channel is dropped). This is already handled by the existing `ConnectionManager` code (line 206-209 in `manager.rs` removes from pending on channel drop). Alternatively, the probe could use a read-only validation that doesn't trigger full handshake -- but reusing the existing protocol is simpler and the cleanup path is already correct.
**Warning signs:** `periphore peers pending` shows entries from probes that connected and disconnected.

### Pitfall 5: flume Receiver Dropped Before mdns-sd Shutdown
**What goes wrong:** If the browse receiver is dropped before calling `mdns.shutdown()`, the mdns-sd daemon thread may panic or leak.
**Why it happens:** mdns-sd uses flume channels internally; dropping the receiver while the sender is still active can cause send errors.
**How to avoid:** Always call `mdns.shutdown()` before dropping the receiver. Use the `CancellationToken` pattern to coordinate: on cancellation, call `mdns.shutdown()`, then let the receiver drop naturally.
**Warning signs:** Thread panic messages from mdns-sd internals on daemon shutdown.

### Pitfall 6: Instant Cannot Be Serialized Over IPC
**What goes wrong:** `std::time::Instant` is monotonic and opaque -- it cannot be meaningfully serialized or sent across processes. IPC responses need `SystemTime` or epoch timestamps.
**Why it happens:** `Instant` is designed for elapsed-time measurement within a single process.
**How to avoid:** Use `Instant` internally for TTL comparison (monotonic, correct for elapsed time). Convert to `SystemTime` or `u64` epoch seconds only when building `DiscoveredPeerInfo` for IPC responses.
**Warning signs:** Serde derive fails on `Instant` fields; or serialized "last_seen" values are meaningless numbers.

## Code Examples

### mDNS Service Registration
```rust
// Source: mdns-sd 0.19.1 docs [CITED: docs.rs/mdns-sd/0.19.1/mdns_sd/]
use mdns_sd::{ServiceDaemon, ServiceInfo};

fn register_service(
    mdns: &ServiceDaemon,
    service_type: &str,
    instance_name: &str,
    port: u16,
) -> anyhow::Result<()> {
    // hostname: use system hostname with ".local." suffix
    let hostname = hostname::get()
        .map(|h| format!("{}.local.", h.to_string_lossy()))
        .unwrap_or_else(|_| "periphore.local.".to_owned());

    // TXT properties: minimal -- just protocol version
    let properties = [("proto_ver", periphore_net::PROTOCOL_VERSION.to_string())];

    let service = ServiceInfo::new(
        service_type,        // "_periphore._tcp.local."
        instance_name,       // e.g., hostname or user-configured name
        &hostname,           // "mymachine.local."
        "",                  // empty string = auto-detect IP
        port,                // 7888 (DEFAULT_PORT)
        &properties[..],
    ).map_err(|e| anyhow::anyhow!("ServiceInfo creation failed: {e}"))?;

    // Enable auto-address detection for multi-homed machines
    let service = service.enable_addr_auto();

    mdns.register(service)
        .map_err(|e| anyhow::anyhow!("mDNS register failed: {e}"))?;

    tracing::info!(service_type, instance_name, port, "mDNS service registered");
    Ok(())
}
```

### DiscoveryConfig Schema Addition
```rust
// Source: periphore-config schema.rs pattern [VERIFIED: codebase]
// Add to periphore-config/src/schema.rs

/// Discovery configuration (Phase 7).
/// Opt-in: disabled by default (D-03, CFG-01).
#[derive(Debug, Deserialize)]
pub struct DiscoveryConfig {
    /// Enable mDNS peer discovery.
    /// Default: false (opt-in per D-03).
    #[serde(default)]
    pub enabled: bool,

    /// Override the mDNS service instance name.
    /// Default: system hostname.
    pub instance_name: Option<String>,

    /// mDNS service type to browse/register.
    /// Default: "_periphore._tcp.local."
    #[serde(default = "default_service_type")]
    pub service_type: String,

    /// Enable SSH tunnel port probing.
    /// Default: false (independent of mDNS enabled).
    #[serde(default)]
    pub ssh_probe_enabled: bool,

    /// Ports to probe for SSH-forwarded Periphore daemons.
    /// Default: [17880, 17881, ..., 17890]
    #[serde(default = "default_ssh_probe_ports")]
    pub ssh_probe_ports: Vec<u16>,
}

fn default_service_type() -> String {
    "_periphore._tcp.local.".to_owned()
}

fn default_ssh_probe_ports() -> Vec<u16> {
    (17880..=17890).collect()
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            instance_name: None,
            service_type: default_service_type(),
            ssh_probe_enabled: false,
            ssh_probe_ports: default_ssh_probe_ports(),
        }
    }
}
```

### IPC Extension (GetDiscoveredPeers)
```rust
// Source: periphore-protocol/src/ipc.rs pattern [VERIFIED: codebase]

// In IpcRequest enum, add:
GetDiscoveredPeers,

// New struct adjacent to PendingPeerInfo:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPeerInfo {
    pub hostname: String,
    pub port: u16,
    /// Seconds since Unix epoch when this peer was last seen.
    pub last_seen_epoch: u64,
    /// Discovery source: "mdns" or "ssh_probe"
    pub source: String,
}

// In IpcResponse enum, add:
DiscoveredPeers {
    peers: Vec<DiscoveredPeerInfo>,
},

// In IpcCommand enum (periphore-ipc/src/lib.rs), add:
GetDiscoveredPeers {
    responder: oneshot::Sender<IpcResponse>,
},

// In IpcCommand::from_request_with_responder, add:
IpcRequest::GetDiscoveredPeers => Self::GetDiscoveredPeers { responder },
```

### CLI Peers Subcommand Group
```rust
// Source: periphore-cli/src/cli.rs pattern [VERIFIED: codebase]

// In Commands enum, add:
/// Manage and inspect peers.
Peers {
    #[command(subcommand)]
    action: PeersAction,
},

/// Sub-actions for `periphore peers`.
#[derive(Subcommand, Debug)]
pub enum PeersAction {
    /// List peers discovered via mDNS or SSH probe.
    Discovered,
    /// List peers awaiting trust verification.
    Pending,
}

// In lib.rs dispatch, add:
cli::Commands::Peers { action } => match action {
    cli::PeersAction::Discovered => commands::peers::discovered::run(&socket_path).await,
    cli::PeersAction::Pending => commands::peers::pending::run(&socket_path).await,
},
```

### Daemon Wiring (periphored/src/main.rs)
```rust
// Source: periphored/src/main.rs pattern [VERIFIED: codebase]

// After ConnectionManager setup, before select! loop:
let mut discovery_service = periphore_discovery::DiscoveryService::new();
let (discovery_event_tx, mut discovery_event_rx) =
    tokio::sync::mpsc::channel::<periphore_discovery::DiscoveryEvent>(64);
let discovery_cancel = tokio_util::sync::CancellationToken::new();

if config.discovery.enabled || config.discovery.ssh_probe_enabled {
    discovery_service.start(
        &mut tasks,
        &config.discovery,
        discovery_event_tx,
        Arc::clone(&identity),
        Arc::clone(&trust_store),
        discovery_cancel.clone(),
    );
    tracing::info!("discovery service started");
}

// In select! loop, add:
// Discovery event
discovery_event = discovery_event_rx.recv() => {
    match discovery_event {
        Some(periphore_discovery::DiscoveryEvent::PeerDiscovered { hostname, port, source }) => {
            tracing::info!(hostname = %hostname, port, source = ?source, "peer discovered");
        }
        Some(periphore_discovery::DiscoveryEvent::PeerRemoved { hostname, port }) => {
            tracing::info!(hostname = %hostname, port, "discovered peer removed");
        }
        Some(periphore_discovery::DiscoveryEvent::Error(msg)) => {
            tracing::warn!(error = %msg, "discovery error");
        }
        None => {
            tracing::debug!("discovery event channel closed");
        }
    }
}

// In IPC dispatch, add:
Some(IpcCommand::GetDiscoveredPeers { responder }) => {
    tracing::debug!("IPC: GetDiscoveredPeers");
    let peers = discovery_service.discovered_list();
    let _ = responder.send(IpcResponse::DiscoveredPeers { peers });
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| libmdns (tokio-coupled) | mdns-sd (runtime-agnostic) | 2023+ | mdns-sd is more actively maintained and runtime-independent |
| simple-mdns | mdns-sd | -- | mdns-sd has better RFC compliance (goodbye packets, conflict resolution) |
| Manual mDNS with raw sockets | mdns-sd crate | -- | Crate handles all RFC 6762/6763 edge cases |
| Port scanning with std::net | tokio::time::timeout + TcpStream | -- | Non-blocking, async, doesn't tie up threads |

**Deprecated/outdated:**
- `trust-dns-proto` for mDNS: superseded by `hickory-dns` rename; `mdns-sd` is the better fit for service discovery specifically
- Synchronous port scanning: blocks the async runtime; use `tokio::time::timeout` with `TcpStream::connect`

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `hostname` crate is available or `gethostname()` syscall works for mDNS instance name | Code Examples (register_service) | Low -- can fall back to "periphore" as default instance name |
| A2 | SSH probe sending Hello to a non-Periphore service on localhost will fail gracefully (timeout or decode error, not crash) | Pattern 3: SSH Probe | Low -- timeout + error handling in probe_handshake covers this |
| A3 | `ServiceInfo::new()` with empty string for IP auto-detects local addresses | Code Examples (register_service) | Medium -- may need to use `enable_addr_auto()` instead; verify in implementation |
| A4 | SSH probe interval of 30 seconds is reasonable (not too aggressive, not too slow) | Pattern 3 | Low -- configurable; 30s matches the GC sweep interval |
| A5 | flume::Receiver::recv_async() is cancel-safe in tokio::select! | Pattern 2 | Medium -- if not cancel-safe, may need to wrap in a pinned future; flume docs suggest it is safe |

## Open Questions (RESOLVED)

1. **Should SSH probe be a separate config flag from mDNS?**
   - What we know: The user specified SSH probing as a secondary mechanism. The two have different failure modes (mDNS needs LAN, SSH probe needs forwarded ports).
   - What's unclear: Whether users want to enable one without the other frequently enough to justify separate flags.
   - Recommendation: Use separate flags (`enabled` for mDNS, `ssh_probe_enabled` for SSH probe). This gives users precise control. Both default to `false`.
   - RESOLVED: Plans use separate `discovery.enabled` (mDNS) and `discovery.ssh_probe_enabled` (SSH probe) flags in `DiscoveryConfig` — both default `false`, independently configurable.

2. **How to handle SSH probe connecting to own daemon on forwarded port?**
   - What we know: If the user has `ssh -R 17888:localhost:7888` running, the local daemon at 7888 is exposed on 17888. The probe would discover itself.
   - What's unclear: Whether fingerprint comparison (probe fingerprint == local fingerprint) is sufficient, or if the daemon's own port should be excluded from probe results.
   - Recommendation: Compare HelloAck fingerprint against local identity. If same fingerprint, skip the entry (it's the local daemon forwarded to itself). This handles all cases including port forwarding loops.
   - RESOLVED: Plan 02 `probe.rs` implements fingerprint comparison in `probe_handshake` — if `HelloAck.fingerprint == identity.fingerprint_hex`, the entry is skipped (self-detection via identity, not port exclusion).

3. **mDNS instance name collision on multi-daemon hosts**
   - What we know: Multiple Periphore daemons on the same host (different ports) would have hostname collisions.
   - What's unclear: Whether this is a real use case in v1.
   - Recommendation: Default to `hostname-{port}` as instance name to avoid collisions. Users can override via `instance_name` config.
   - RESOLVED: Plan 01 adds `instance_name: Option<String>` to `DiscoveryConfig`; `mdns.rs` defaults to `hostname-{port}` format when not set, with user override available.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Build | Yes | 1.95.0 | -- |
| cargo | Build | Yes | 1.95.0 | -- |
| mdns-sd crate | mDNS discovery | Yes (crates.io) | 0.19.1 | Manual `[[peer]]` config |
| Network (multicast) | mDNS | Depends on network | -- | SSH probe + manual config |
| SSH tunnels | SSH probe | Depends on user setup | -- | mDNS + manual config |

**Missing dependencies with no fallback:** None -- all critical dependencies available.

**Missing dependencies with fallback:**
- Multicast network access (mDNS): falls back to SSH probe and manual config
- SSH tunnel setup (SSH probe): falls back to mDNS and manual config

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in Rust test harness) |
| Config file | Per-crate Cargo.toml `[lib] test = false` + `tests/` subdir |
| Quick run command | `cargo test -p periphore-discovery` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| NET-02 SC1 | Daemon broadcasts via mDNS and appears in another daemon's peer list within 5 seconds | integration | `cargo test -p periphore-discovery -- mdns_register_browse` | No -- Wave 0 |
| NET-02 SC2 | Discovered peers proceed through identity verification handshake | integration | `cargo test -p periphore-net -- handshake` (existing) + future Phase 8 wiring | Partial (handshake tests exist) |
| NET-02 SC3 | mDNS failure logs warning, manual config works | unit | `cargo test -p periphore-discovery -- mdns_bind_failure` | No -- Wave 0 |
| NET-02-SSH | SSH probe discovers forwarded Periphore daemon | integration | `cargo test -p periphore-discovery -- ssh_probe_discovers_daemon` | No -- Wave 0 |
| NET-02-SSH | SSH probe skips non-Periphore services | unit | `cargo test -p periphore-discovery -- ssh_probe_non_periphore` | No -- Wave 0 |
| NET-02-SSH | SSH probe skips own daemon (fingerprint match) | unit | `cargo test -p periphore-discovery -- ssh_probe_self_detection` | No -- Wave 0 |
| D-07/D-08 | TTL GC removes stale entries after 5 minutes | unit | `cargo test -p periphore-discovery -- gc_removes_expired` | No -- Wave 0 |
| D-09 | Peer list caps at 64 entries, evicts oldest | unit | `cargo test -p periphore-discovery -- list_cap_eviction` | No -- Wave 0 |
| D-10 | CLI `periphore peers discovered` displays table | integration | `cargo test -p periphore-cli -- peers_discovered` | No -- Wave 0 |
| D-11 | CLI `periphore peers pending` displays pending peers | integration | `cargo test -p periphore-cli -- peers_pending` | No -- Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p periphore-discovery && cargo test -p periphore-config && cargo build --workspace`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/periphore-discovery/tests/integration.rs` -- covers NET-02 SC1, SC3, SSH probe
- [ ] `crates/periphore-discovery/Cargo.toml` -- new crate scaffold with `[lib] test = false`
- [ ] `crates/periphore-discovery/src/lib.rs` -- crate root
- [ ] Framework install: none needed (cargo test built-in)

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | -- (discovery is passive, no auth) |
| V3 Session Management | No | -- |
| V4 Access Control | No | -- (discovered peers are not connected) |
| V5 Input Validation | Yes | Validate mDNS ServiceInfo fields (hostname, port) before storing; validate SSH probe HelloAck before trusting |
| V6 Cryptography | No | -- (no crypto in discovery; handshake reuse sends fingerprint but does not establish encrypted channel) |

### Known Threat Patterns for mDNS + Port Probing

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| mDNS spoofing: attacker broadcasts fake `_periphore._tcp.local.` | Spoofing | Discovery is passive -- no auto-connect. Actual connection goes through full identity handshake (Phase 6 SEC-01/SEC-05/SEC-06). Spoofed discovery entries are harmless. |
| mDNS flooding: attacker broadcasts many services to fill 64-peer cap | Denial of Service | D-09 cap (64 peers) limits memory; oldest-eviction prevents unbounded growth. `tracing::warn!` on eviction alerts operators. |
| SSH probe port confusion: non-Periphore service on probe port | Information Disclosure | Probe sends Hello with local fingerprint to unknown services. Risk is minimal: fingerprint is public key hash (not secret). Probe timeout prevents hanging. |
| Probe scan detection: security tools flag rapid localhost connections | Denial of Service | 30-second probe interval with 11 ports = one connection every ~3 seconds. Not aggressive enough to trigger IDS. All connections are to localhost only. |

## Sources

### Primary (HIGH confidence)
- mdns-sd 0.19.1 crate documentation [CITED: docs.rs/mdns-sd/0.19.1/mdns_sd/]
- mdns-sd GitHub repository [CITED: github.com/keepsimple1/mdns-sd]
- Periphore codebase: `crates/periphore-net/src/manager.rs`, `handshake.rs`, `codec.rs` [VERIFIED: codebase]
- Periphore codebase: `crates/periphore-config/src/schema.rs`, `lib.rs` [VERIFIED: codebase]
- Periphore codebase: `crates/periphore-protocol/src/ipc.rs`, `peer.rs` [VERIFIED: codebase]
- Periphore codebase: `crates/periphore-cli/src/lib.rs`, `cli.rs`, `commands/` [VERIFIED: codebase]
- Periphore codebase: `crates/periphored/src/main.rs` [VERIFIED: codebase]
- `cargo search mdns-sd` version verification [VERIFIED: cargo registry]

### Secondary (MEDIUM confidence)
- mdns-sd ServiceDaemon, ServiceEvent, ServiceInfo, ResolvedService API docs [CITED: docs.rs/mdns-sd/0.19.1/]
- flume async compatibility [CITED: docs.rs/flume/latest/flume/struct.Receiver.html]
- tokio TcpStream::connect timeout pattern [CITED: docs.rs/tokio/latest/tokio/net/struct.TcpStream.html]

### Tertiary (LOW confidence)
- mdns-sd GitHub issues for macOS/firewall behavior [CITED: github.com/keepsimple1/mdns-sd/issues]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- `mdns-sd` specified in CLAUDE.md, version verified via cargo registry, API verified via docs.rs
- Architecture: HIGH -- all integration points verified against existing codebase patterns
- Pitfalls: HIGH -- pitfalls 1-2 verified via mdns-sd docs; pitfalls 3-6 derived from codebase analysis of ConnectionManager behavior
- SSH probe design: MEDIUM -- probe pattern is straightforward but exact interaction with remote ConnectionManager pending state needs implementation-time validation (Pitfall 4 mitigation verified correct by reading manager.rs cleanup code)

**Research date:** 2026-04-28
**Valid until:** 2026-05-28 (30 days -- stable domain, mdns-sd changes slowly)
