# Architecture Patterns — Periphore

**Researched:** 2026-04-22
**Confidence:** MEDIUM — based on Synergy/Barrier/Input Leap open-source analysis; crate API surfaces should be re-verified before implementation.

---

## 1. How Synergy/Barrier/Input Leap Are Structured

All three share essentially the same architecture (Barrier forked from Synergy; Input Leap forked from Barrier).

### Server (primary machine — physical keyboard/mouse attached)
- Runs platform-specific input capture (CGEventTap on macOS, X11 grabs on Linux, Win32 hooks on Windows)
- Holds the complete screen map: which client is on which edge of which other client
- Main event loop: capture → decide if edge crossing → forward to target client
- Listens on TCP port 24800

### Client (secondary machines)
- Connects to server TCP port
- Reports its own screen dimensions to the server
- Receives input events, injects them locally
- Has no awareness of other clients; everything is mediated by the server

### Synergy Wire Protocol
Binary framed over TCP: `4-byte big-endian length | 4-byte ASCII command | payload`

Key message types: `CINN` (cursor enters), `COUT` (cursor leaves), `DMMV` (mouse move), `DKDN`/`DKUP` (key), `DINF` (client info), `DCLP` (clipboard).

### Observations That Matter for Periphore

1. **Server is the brain.** Topology knowledge, routing decisions, and edge detection are centralized. This is architecturally incompatible with P2P.
2. **Input capture and transport are tightly coupled.** Hard to test without a real network peer.
3. **The protocol is stateful but unversioned.** No forward-compatibility story.

---

## 2. What Changes for P2P

| Concern | Synergy/Barrier | Periphore |
|---------|-----------------|-----------|
| Topology knowledge | Server only | Every peer holds complete map |
| Edge crossing decision | Server detects, routes | Source peer decides locally |
| Input forwarding | Server → specific client | Source peer → specific peer directly |
| Connection topology | Star (all clients → server) | Full mesh (every peer ↔ every peer) |
| Role assignment | Static: server captures, client injects | Dynamic: any peer can be source or sink |
| Focus tracking | Single authoritative state on server | Token-passing: one peer holds focus at a time |
| Failure mode | Server dies = everything stops | One peer dies = only its edges go offline |

### Focus Token Model

A "focus token" is held by exactly one peer at any time. When the cursor crosses an edge:
1. Current holder sends `FocusTransfer` to target peer
2. Current holder transitions from Active → Idle
3. Target peer receives `FocusTransfer`, transitions to Sink
4. Physical cursor movement is the natural serialization point — race conditions are human-speed events

**Recovery:** If the focus-holding peer dies, the local machine reclaims its input after a timeout (no `Pong` received for N seconds → reset to Active).

---

## 3. Crate Workspace Structure

```
periphore/
  Cargo.toml                  (workspace)
  crates/
    periphore-protocol/        (shared message types — ZERO runtime deps)
    periphore-config/          (TOML config parsing)
    periphore-identity/        (keypair, fingerprint, identicon, word phrase)
    periphore-core/            (state machine, topology, routing — ZERO platform deps)
    periphore-ipc/             (Unix socket server, JSON-lines protocol)
    periphore-net/             (TCP transport, framing, peer handshake)
    periphore-capture/         (platform input capture — cfg-gated)
    periphore-inject/          (platform input injection — cfg-gated)
    periphored/                (daemon binary entry — thin main.rs, orchestrates all functional crates)
    periphore/                 (CLI binary entry — thin main.rs, calls periphore-cli library)
    periphore-cli/             (CLI support library — client-specific logic, no main)
```

### Dependency Graph

```
periphore-protocol   (shared types, no internal deps)
  ├── periphore-core
  ├── periphore-net
  ├── periphore-ipc
  ├── periphore-capture
  ├── periphore-inject
  └── periphore-cli

periphore-config     (no internal deps)
periphore-identity   (no internal deps)

periphored (binary)  (depends on all functional crates, orchestrates daemon)
periphore  (binary)  (depends on periphore-cli, thin CLI entry)
```

**Critical insight:** `periphore-core` has zero platform dependencies. It is pure logic — the state machine, topology resolver, and routing algorithm. This is the most important crate and the most testable.

---

## 4. Concurrency Model: Channel-Based Tasks

**Recommendation:** `tokio::mpsc` channel-based message passing. NOT actor frameworks. NOT shared state on the hot path.

**Why not actor frameworks (Actix, Kameo):** Over-engineered for this domain. The message patterns are simple: unidirectional event streams and request/response for IPC. Actors add supervisor trees and mailbox semantics without solving a real problem here.

**Why not `Arc<Mutex<...>>`:** Creates contention on the hot path. Mouse events arrive at 1000+ Hz. Lock acquisition on every event is unacceptable.

### Channel Topology

```
capture_task ──mpsc──> router_task ──mpsc──> inject_task
                           │   ^
                           │   │
                    mpsc   │   │  mpsc
                           ▼   │
                     net_send_task (per peer)
                     net_recv_task (per peer) ──mpsc──> router_task
                           │
                    mpsc   │
                           ▼
                       ipc_task ◄──mpsc──> router_task
```

All channels are **bounded** (input events: ~256, control: ~64). This provides backpressure: if injection can't keep up, the source slows.

### Task Structure

```rust
async fn main() {
    // Create channels
    let (capture_tx, capture_rx) = mpsc::channel::<InputEvent>(256);
    let (inject_tx, inject_rx)   = mpsc::channel::<InputEvent>(256);
    let (ipc_cmd_tx, ipc_cmd_rx) = mpsc::channel::<IpcCommand>(64);

    let mut tasks = JoinSet::new();
    tasks.spawn(capture::run(capture_tx));
    tasks.spawn(inject::run(inject_rx));
    tasks.spawn(router::run(capture_rx, inject_tx, ipc_cmd_rx, ...));
    tasks.spawn(ipc::run(ipc_cmd_tx, ...));
    tasks.spawn(net::listener(...));

    tokio::select! {
        _ = signal::ctrl_c() => { /* graceful shutdown */ },
        result = tasks.join_next() => { /* handle task crash */ },
    }
}
```

---

## 5. Wire Protocol Design

### Frame Structure

```
+----------------+----------+---------+
| Length (4B BE) | MsgType  | Payload |
+----------------+----------+---------+
```

**Serialization:** `postcard` + `serde`. Compact varint encoding saves bandwidth on high-frequency mouse events. Use `tokio_util::codec::LengthDelimitedCodec` for framing; implement a custom `Codec` wrapping it.

### Message Type Sketch

```rust
enum PeerMessage {
    // Handshake
    Hello { protocol_version: u32, fingerprint: [u8; 32], public_key: Vec<u8> },
    HelloAck { fingerprint: [u8; 32], public_key: Vec<u8>, accepted: bool },

    // Topology
    TopologyAdvertise { monitors: Vec<MonitorInfo> },
    TopologyPropose { edges: Vec<EdgeMapping> },
    TopologyAccept,
    TopologyReject { reason: String },

    // Focus
    FocusTransfer { entry_edge: Edge, entry_position: f64, sequence: u64 },
    FocusAck { sequence: u64 },
    FocusReclaim,

    // Input
    MouseMove { dx: i32, dy: i32 },
    MouseButton { button: u8, pressed: bool },
    MouseScroll { dx: i32, dy: i32 },
    KeyEvent { scancode: u32, pressed: bool, modifiers: u8 },

    // Control
    Ping { timestamp: u64 },
    Pong { timestamp: u64 },
    Bye,
}
```

### Handshake Sequence

```
Peer A                                  Peer B
  |──── TCP connect ──────────────────>  |
  |──── Hello { fp_A, pubkey_A } ──────> |
  |<─── HelloAck { fp_B, pubkey_B } ─── |
  |──── HelloAck { accepted: true } ───> |
  |──── TopologyAdvertise ─────────────> |
  |<─── TopologyAdvertise ────────────── |
  |──── TopologyPropose { edges } ─────> |
  |<─── TopologyAccept ────────────────  |
  |  [ready for input events]            |
```

**On first connection:** Fingerprints are unknown. Daemon surfaces pending verification via IPC. CLI user confirms (identicon comparison or word-phrase entry). Accepted fingerprints cached. Future connections verified automatically.

---

## 6. Monitor Topology Algorithm

### Edge Mapping Structure

```rust
struct ResolvedEdge {
    from_monitor: MonitorId,     // (peer_fingerprint, monitor_id)
    from_edge:    Edge,          // Left | Right | Top | Bottom
    from_range:   (u32, u32),    // pixel range along source edge
    to_monitor:   MonitorId,
    to_edge:      Edge,
    to_range:     (u32, u32),    // pixel range along target edge
}
```

### Cursor-at-Edge Resolution

```
1. Cursor (x, y) hits edge E of monitor M
2. Compute position along edge:
     Left/Right → pos = y;  Top/Bottom → pos = x
3. Find ResolvedEdge where from == M, from_edge == E, pos ∈ from_range
4. Map pos into to_range (proportional)
5. Convert to_pos → (entry_x, entry_y) based on to_edge:
     Left  → x=0,         y=to_pos
     Right → x=width-1,   y=to_pos
     Top   → x=to_pos,    y=0
     Bottom→ x=to_pos,    y=height-1
6. Emit FocusTransfer { entry_edge: to_edge, entry_position: normalized }
```

### Offset Compensation (Mismatched Monitor Heights)

Default strategy: **Centered alignment**. The shorter edge's range maps to the center of the taller edge. Areas outside the overlap are dead zones — cursor bumps that portion of the edge.

```
Monitor A (height=1080)  |  Monitor B (height=1440)
                         |
A.right [0..1080]  maps to  B.left [180..1260]  (centered)
```

Strategy is configurable per edge: `top-aligned`, `centered`, `bottom-aligned`, or explicit pixel range.

---

## 7. IPC Design for Testability

**Protocol:** JSON-lines over Unix domain socket (newline-delimited JSON). Local-only so human-readable format is fine.

```rust
enum IpcRequest {
    GetStatus,
    ListPeers,
    GetTopology,
    AcceptFingerprint { fingerprint: String },
    RejectFingerprint { fingerprint: String },
    ReloadConfig,
    // Testing:
    InjectInputEvent { event: InputEvent },
    SimulateEdgeCross { edge: Edge, position: f64 },
    GetState,
    GetPendingVerifications,
    GetIdenticon { fingerprint: String },
    GetWordPhrase { fingerprint: String },
}
```

**Test harness pattern:** The daemon can run with capture, injection, and network all disabled. Events enter via IPC `InjectInputEvent`, routing decisions are observable via `GetState`. This means `periphore-core` can be tested end-to-end without any platform code.

**Socket location:**
- Linux: `$XDG_RUNTIME_DIR/periphore/periphore.sock`
- macOS: `$TMPDIR/periphore/periphore.sock`
- Permissions: `0600`

---

## 8. Build Order

| Phase | Crates | Rationale |
|-------|--------|-----------|
| 1 | `protocol`, `config`, `identity` | Shared vocabulary; everything depends on these |
| 2 | `core`, `ipc`, `ctl` | State machine + control interface; enables full unit/integration testing |
| 3 | `net` | TCP transport; can be tested with IPC-injected events over real TCP |
| 4 | `capture`, `inject` | Platform input; first "it actually works" milestone |
| 5 | Seamless upgrade to `capture` | CGEventTap / evdev grab; deferred per project constraints |

---

## 9. Patterns to Follow

**Trait-based platform abstraction:**
```rust
#[async_trait]
pub trait InputCapture: Send + Sync + 'static {
    async fn start(&mut self, tx: mpsc::Sender<InputEvent>) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    fn supports_seamless(&self) -> bool;
}
```

**Purely functional state machine** (input → output actions, no side effects inside):
```rust
impl Router {
    pub fn handle_event(&mut self, event: RouterEvent) -> Vec<RouterAction> {
        match (&self.role, event) { /* transitions */ }
    }
}
```

---

## 10. Anti-Patterns to Avoid

| Anti-Pattern | Why Bad | Instead |
|-------------|---------|---------|
| Couple capture to transport | Untestable without real network | Capture sends to channel; router decides |
| Shared mutable topology state | Race conditions with edge crossing | Router owns topology; updates arrive as messages |
| Blocking in async context | Starves tokio runtime | `spawn_blocking` for CPU work |
| Unbounded channels | Memory leak, unbounded latency | Bounded channels + mouse-move coalescing |
| Panic on bad peer data | One bad peer crashes daemon | Per-connection error boundaries; only fatal on config/socket failures |

---

## 11. Security Architecture

- **Ed25519** — identity keypairs (via `ed25519-dalek`)
- **rustls** — TLS 1.3 session encryption (self-signed cert derived from Ed25519 key)
- **SHA-256** — fingerprint derivation from public key
- Fingerprints cached in `$XDG_CACHE_HOME/periphore/known_peers.toml` (never in main config)
- Daemon surfaces pending verifications via IPC; no automatic acceptance
- Hard config fingerprint pinning: unknown peers refused without IPC confirmation
