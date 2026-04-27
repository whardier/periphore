---
phase: "06"
plan: "06-03"
subsystem: tcp-peering
tags: [tcp, handshake, connection-manager, exponential-backoff, cancellation-token, identity, fingerprint]
dependency_graph:
  requires:
    - "06-01 (periphore-net Cargo.toml deps, DaemonConfig.listen)"
    - "06-02 (NetError, codec, PeerEvent, HandshakeResult, PendingPeer, ConnectionControl)"
  provides:
    - identicon_from_fingerprint free function in periphore-identity (D-02)
    - word_phrase_from_fingerprint free function in periphore-identity (D-02)
    - PROTOCOL_VERSION = 1 constant in handshake.rs
    - perform_handshake_initiator (outbound connector side)
    - perform_handshake_responder (inbound accept side)
    - ConnectionManager struct with spawn_listener, spawn_connector, promote_pending, pending_list, cancel_peer
    - Exponential backoff connector (1s->30s cap, D-09)
    - CancellationToken per connector (T-6-05)
    - Arc<Mutex<HashMap>> shared pending state
  affects:
    - crates/periphore-identity/src/lib.rs
    - crates/periphore-net/src/handshake.rs
    - crates/periphore-net/src/manager.rs
    - crates/periphore-net/src/lib.rs
    - crates/periphored/src/main.rs (06-04 will wire ConnectionManager)
tech_stack:
  added:
    - anyhow dep in periphore-net (for JoinSet<anyhow::Result<()>> type compatibility with periphored)
    - futures-util sink feature in workspace Cargo.toml (for SinkExt on FramedWrite)
  patterns:
    - tokio::time::timeout wrapping every FramedRead::next() — hung peer protection
    - Arc<Mutex<HashMap>> shared between manager methods and spawned tasks
    - CancellationToken + tokio::select! for prompt task cancellation
    - TCP_NODELAY immediately after accept()/connect() before split_framed() (D-19 hard requirement)
    - identicon_from_fingerprint/word_phrase_from_fingerprint free functions for peer fingerprints
key_files:
  created:
    - crates/periphore-net/src/handshake.rs
    - crates/periphore-net/src/manager.rs
  modified:
    - crates/periphore-identity/src/lib.rs
    - crates/periphore-net/src/lib.rs
    - crates/periphore-net/Cargo.toml
    - Cargo.toml (workspace futures-util sink feature)
key_decisions:
  - "anyhow added as dep to periphore-net for JoinSet<anyhow::Result<()>> type — spawn_listener/spawn_connector take &mut JoinSet to match periphored's tasks type exactly"
  - "futures-util sink feature required for SinkExt on FramedWrite — workspace dep updated from version-only to features=[sink]"
  - "HandshakeResult imported from crate::connection in manager.rs (not via handshake re-export) — it is defined in connection.rs, not handshake.rs"
  - "perform_handshake_* kept as pub (not pub(crate)) — plan acceptance criteria grep for pub async fn; unreachable_pub warning is a warn-level lint that doesn't block build"
  - "active HashMap field retained in ConnectionManager despite dead_code warning — Phase 9 adds input forwarding channel here; intentional placeholder"

requirements-completed:
  - NET-01
  - NET-03
  - NET-04

duration: 4min
completed: "2026-04-27"
---

# Phase 6 Plan 3: periphore-net handshake.rs + manager.rs + lib.rs completion Summary

**Hello/HelloAck handshake protocol with 10s timeout, version/fingerprint checks, trust lookup, and ConnectionManager with exponential backoff connector (1s→30s), CancellationToken cancellation, and Arc-shared pending state — periphore-net crate fully implemented.**

## Performance

- **Duration:** 4 min
- **Started:** 2026-04-27T09:07:32Z
- **Completed:** 2026-04-27T09:11:00Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Added `identicon_from_fingerprint` and `word_phrase_from_fingerprint` public free functions to `periphore-identity` — allows handshake.rs to compute visual/verbal identifiers for PEER fingerprints without a keypair (D-02)
- Implemented complete Hello/HelloAck protocol in `handshake.rs` with 10-second receive timeout (T-6-02), protocol version mismatch detection (T-6-03), fingerprint conflict detection (T-6-04), and trust store lookup returning `HandshakeResult::Trusted`, `::Pending`, or `::Rejected`
- Implemented `ConnectionManager` in `manager.rs` with accept loop (TCP_NODELAY-first), outbound retry connector with 1s→2s→4s→8s→16s→30s cap backoff (D-09), `CancellationToken` per connector (T-6-05), `Arc<Mutex<HashMap>>` shared pending state, and all five public methods: `spawn_listener`, `spawn_connector`, `promote_pending`, `pending_list`, `cancel_peer`
- `cargo build --workspace` exits 0; all 20 periphore-net tests (from Plan 02) continue passing

## Task Commits

1. **Task 1: handshake.rs + identity free functions** — `f827506` (feat)
2. **Task 2: manager.rs + lib.rs completion** — `30b2949` (feat)

## Files Created/Modified

- `crates/periphore-identity/src/lib.rs` — added `identicon_from_fingerprint` and `word_phrase_from_fingerprint` public free functions (D-02)
- `crates/periphore-net/src/handshake.rs` — PROTOCOL_VERSION=1, perform_handshake_initiator, perform_handshake_responder
- `crates/periphore-net/src/manager.rs` — ConnectionManager with all five methods, backoff constants, Arc<Mutex<HashMap>> pending state
- `crates/periphore-net/src/lib.rs` — added `mod handshake; mod manager;` and `pub use manager::ConnectionManager; pub use handshake::PROTOCOL_VERSION;`
- `crates/periphore-net/Cargo.toml` — added `anyhow = { workspace = true }` dep
- `Cargo.toml` — added `sink` feature to `futures-util` workspace dep

## Decisions Made

- `anyhow` added as a dep to `periphore-net` so `spawn_listener`/`spawn_connector` can accept `&mut JoinSet<anyhow::Result<()>>` matching `periphored`'s existing tasks type exactly — avoids needing a separate `JoinSet` type at the wiring layer.
- `futures-util` sink feature added to the workspace `Cargo.toml` declaration — `SinkExt::send()` is gated behind the `sink` feature; without it `FramedWrite::send()` does not resolve.
- `HandshakeResult` imported from `crate::connection` (not via `crate::handshake`) in manager.rs — the type is defined in `connection.rs`; the `use` in `handshake.rs` is a local import, not a re-export.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `sink` feature to futures-util workspace dep**
- **Found during:** Task 1 (handshake.rs build)
- **Issue:** `futures_util::SinkExt` is gated behind the `sink` feature; workspace dep had no features, causing `SinkExt::send()` to be unresolvable on `FramedWrite`
- **Fix:** Changed `futures-util = { version = "0.3" }` to `futures-util = { version = "0.3", features = ["sink"] }` in root `Cargo.toml`
- **Files modified:** `Cargo.toml`, `Cargo.lock`
- **Verification:** `cargo build -p periphore-net` exits 0 after fix
- **Committed in:** `f827506` (Task 1 commit)

**2. [Rule 3 - Blocking] Added `anyhow` dep to periphore-net**
- **Found during:** Task 2 (manager.rs build)
- **Issue:** `manager.rs` uses `JoinSet<anyhow::Result<()>>` in `spawn_listener`/`spawn_connector` signatures (to match periphored's tasks type), but `anyhow` was not in `periphore-net/Cargo.toml`
- **Fix:** Added `anyhow = { workspace = true }` to `periphore-net/Cargo.toml`
- **Files modified:** `crates/periphore-net/Cargo.toml`, `Cargo.lock`
- **Verification:** `cargo build --workspace` exits 0 after fix
- **Committed in:** `30b2949` (Task 2 commit)

**3. [Rule 1 - Bug] Fixed HandshakeResult import path in manager.rs**
- **Found during:** Task 2 (manager.rs build)
- **Issue:** Plan showed `handshake::{self, HandshakeResult}` but `HandshakeResult` is defined in `crate::connection`, not re-exported from `handshake`
- **Fix:** Changed import to `crate::connection::{..., HandshakeResult, ...}` and `handshake` imported separately
- **Files modified:** `crates/periphore-net/src/manager.rs`
- **Verification:** `cargo build -p periphore-net` exits 0 after fix
- **Committed in:** `30b2949` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (2 blocking, 1 bug)
**Impact on plan:** All three fixes were direct build blockers caused by the current task's code. No scope creep.

## Issues Encountered

- `unreachable_pub` lint fires on `perform_handshake_*` functions because they are `pub` but not re-exported from the crate root (only called by `manager.rs` within the crate). This is `warn` level, not `deny` — build succeeds. The functions are kept `pub` per plan acceptance criteria which grep for `pub async fn`. Future plans may add re-exports or change to `pub(crate)`.

## Known Stubs

- `active: HashMap<String, ActiveConn>` field in `ConnectionManager` is never written to — Phase 9 adds the input forwarding channel. This is intentional; the field is a placeholder for Phase 9.
- Phase 6 read loops in both `spawn_listener` and `spawn_connector` (after a trusted handshake) use `// Ignore non-handshake frames in Phase 6` — Phase 9 wires real input event dispatch here.

## Threat Surface Scan

| Mitigated | File | Description |
|-----------|------|-------------|
| T-6-02 | handshake.rs | `tokio::time::timeout(10s, framed_read.next())` on both responder and initiator receive steps — hung/malicious peer cannot block task indefinitely |
| T-6-03 | handshake.rs | `if protocol_version != PROTOCOL_VERSION` → send `HelloAck { accepted: false }` → return `Err(NetError::ProtocolVersion)` — connection task terminates |
| T-6-04 | handshake.rs | `periphore_trust::check_peer_fingerprint()` called when `PeerConfig.fingerprint` is set; conflict → send `HelloAck { accepted: false }` → return `Err(NetError::FingerprintConflict)` |
| T-6-05 | manager.rs | Every backoff sleep in connector retry loop is inside `tokio::select! { _ = token.cancelled() => return Ok(()) }` — removed peers' tasks exit promptly via `cancel_peer()` |
| D-19 | manager.rs | `stream.set_nodelay(true)` is the first operation after both `accept()` (line 99) and `TcpStream::connect()` (line 271) — before `split_framed()`, before any data exchange |

## Self-Check: PASSED

- `crates/periphore-identity/src/lib.rs`: `pub fn identicon_from_fingerprint` — FOUND
- `crates/periphore-identity/src/lib.rs`: `pub fn word_phrase_from_fingerprint` — FOUND
- `crates/periphore-net/src/handshake.rs`: `pub const PROTOCOL_VERSION: u32 = 1` — FOUND
- `crates/periphore-net/src/handshake.rs`: `pub async fn perform_handshake_initiator` — FOUND
- `crates/periphore-net/src/handshake.rs`: `pub async fn perform_handshake_responder` — FOUND
- `crates/periphore-net/src/handshake.rs`: `tokio::time::timeout` — FOUND (both functions)
- `crates/periphore-net/src/handshake.rs`: `NetError::ProtocolVersion` — FOUND
- `crates/periphore-net/src/handshake.rs`: `accepted: false` — FOUND (multiple occurrences)
- `crates/periphore-net/src/handshake.rs`: `NetError::FingerprintConflict` — FOUND
- `crates/periphore-net/src/handshake.rs`: `identicon_from_fingerprint` — FOUND
- `crates/periphore-net/src/handshake.rs`: `word_phrase_from_fingerprint` — FOUND
- `crates/periphore-net/src/manager.rs`: `pub struct ConnectionManager` — FOUND
- `crates/periphore-net/src/manager.rs`: `set_nodelay(true)` — FOUND at lines 99 and 271 (2 occurrences)
- `crates/periphore-net/src/manager.rs`: `BACKOFF_INITIAL_MS: u64 = 1_000` — FOUND
- `crates/periphore-net/src/manager.rs`: `BACKOFF_CAP_MS: u64 = 30_000` — FOUND
- `crates/periphore-net/src/manager.rs`: `token.cancelled()` — FOUND
- `crates/periphore-net/src/manager.rs`: `pub fn spawn_listener` — FOUND
- `crates/periphore-net/src/manager.rs`: `pub fn spawn_connector` — FOUND
- `crates/periphore-net/src/manager.rs`: `pub async fn promote_pending` — FOUND
- `crates/periphore-net/src/manager.rs`: `pub fn pending_list` — FOUND
- `crates/periphore-net/src/manager.rs`: `pub fn cancel_peer` — FOUND
- `crates/periphore-net/src/lib.rs`: `pub use manager::ConnectionManager` — FOUND
- `crates/periphore-net/src/lib.rs`: `pub use handshake::PROTOCOL_VERSION` — FOUND
- Commit `f827506`: FOUND
- Commit `30b2949`: FOUND
- `cargo build -p periphore-identity`: exits 0
- `cargo build -p periphore-net`: exits 0
- `cargo build --workspace`: exits 0
- `cargo test --workspace`: all pass (20 periphore-net tests + all prior)
