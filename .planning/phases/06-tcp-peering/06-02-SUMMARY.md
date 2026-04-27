---
phase: "06"
plan: "06-02"
subsystem: "periphore-net foundational types"
tags: [tcp-peering, codec, error, types, tdd]
dependency_graph:
  requires:
    - "06-01 (periphore-net Cargo.toml deps)"
  provides:
    - NetError enum (8 variants)
    - split_framed() codec function
    - encode_message() / decode_message() codec functions
    - MAX_FRAME_LENGTH constant (64*1024)
    - PeerEvent enum
    - HandshakeResult enum
    - PendingPeer struct
    - ActiveConn struct
    - ConnectionControl enum
    - DEFAULT_PORT = 7888
  affects:
    - crates/periphore-net/src/error.rs
    - crates/periphore-net/src/codec.rs
    - crates/periphore-net/src/event.rs
    - crates/periphore-net/src/connection.rs
    - crates/periphore-net/src/lib.rs
tech_stack:
  added: []
  patterns:
    - thiserror-derived error enum (consistent with TrustError pattern)
    - LengthDelimitedCodec builder with max_frame_length for OOM protection
    - postcard to_allocvec + from_bytes for PeerMessage framing
    - mpsc::Sender<ConnectionControl> for pending peer promotion/rejection
    - TDD red/green/refactor per-task cycle with integration tests in tests/ subdir
key_files:
  created:
    - crates/periphore-net/src/error.rs
    - crates/periphore-net/src/codec.rs
    - crates/periphore-net/src/event.rs
    - crates/periphore-net/src/connection.rs
    - crates/periphore-net/tests/codec_error.rs
    - crates/periphore-net/tests/event_connection.rs
  modified:
    - crates/periphore-net/src/lib.rs
decisions:
  - "Two separate LengthDelimitedCodec instances in split_framed() — LengthDelimitedCodec does not implement Clone"
  - "MAX_FRAME_LENGTH exported as pub const for use in tests and future modules"
  - "codec module declared pub (pub mod codec) so tests can import codec::split_framed/encode_message/decode_message"
  - "event and connection modules declared as private mod — all types re-exported from crate root via pub use"
  - "HandshakeResult::Pending state is the T-6-02 mitigation: unknown peers cannot reach ActiveConn without ConnectionControl::PromoteTrusted"
metrics:
  duration_minutes: 4
  completed_date: "2026-04-27"
  tasks_completed: 2
  files_changed: 7
---

# Phase 6 Plan 2: periphore-net error, codec, event, connection types Summary

**One-liner:** NetError (8 variants) + LengthDelimitedCodec/postcard framing (64KB OOM cap) + PeerEvent/HandshakeResult/PendingPeer/ActiveConn/ConnectionControl pure data types — full contract surface for handshake.rs and manager.rs in Plan 03.

## What Was Built

This plan created the four foundational modules of `periphore-net` — pure data and transform types with no async task spawning and no TCP sockets open. All contracts that Plans 03 and 04 build against are now defined.

### 1. error.rs — NetError enum

8 variants via `thiserror` (consistent with `TrustError` in `periphore-trust`):

| Variant | Description |
|---------|-------------|
| `Io(#[from] std::io::Error)` | Wraps TCP I/O errors via From impl |
| `Encode(String)` | postcard serialization failure |
| `Decode(String)` | postcard deserialization failure |
| `ConnectionClosed` | Peer closed before handshake completed |
| `UnexpectedMessage(String)` | Wrong message type at handshake step |
| `FingerprintConflict(String)` | SEC-06 violation — known peer presents wrong fingerprint |
| `ProtocolVersion { expected, got }` | Protocol version mismatch |
| `PeerNotFound(String)` | promote_pending() for non-pending fingerprint |

### 2. codec.rs — framing and serialization

- `MAX_FRAME_LENGTH: usize = 64 * 1024` — T-6-01 mitigation (OOM prevention)
- `split_framed(TcpStream)` — splits into `(FramedRead, FramedWrite)` with two separate `LengthDelimitedCodec` instances (not cloneable); D-18 (4-byte big-endian length header)
- `encode_message(&PeerMessage) -> Result<Bytes, NetError>` — postcard `to_allocvec`
- `decode_message(BytesMut) -> Result<PeerMessage, NetError>` — postcard `from_bytes`
- Safety comment: CALLER RESPONSIBILITY — TCP_NODELAY must be set before calling `split_framed()`

### 3. event.rs — PeerEvent enum

One-way notification channel from `ConnectionManager` to the daemon's `select!` loop:

- `PeerPending { fingerprint, identicon, word_phrase }` — unknown peer held pending
- `PeerConnected { peer_id: PeerId }` — trusted peer connected
- `PeerDisconnected { peer_id: PeerId }` — established connection dropped

### 4. connection.rs — connection state types

Pure data types for the connection state machine:

- `ConnectionControl` enum: `PromoteTrusted` | `Reject` (daemon → pending task)
- `PendingPeer`: fingerprint_hex, identicon, word_phrase, `promote_tx: mpsc::Sender<ConnectionControl>`
- `ActiveConn`: peer_id (Phase 9 adds input forwarding channel)
- `HandshakeResult`: `Trusted { peer_id, fingerprint_hex }` | `Pending { peer_id, fingerprint_hex, identicon, word_phrase }` | `Rejected { reason }`

### 5. lib.rs — crate root updated

Replaced 2-line stub with full crate root: declares all four modules, re-exports all public types, defines `DEFAULT_PORT = 7888`.

## Tests Added

| Test File | Test Name | Covers |
|-----------|-----------|--------|
| `tests/codec_error.rs` | `net_error_io_wraps_std_io_error` | NetError::Io From<io::Error> conversion |
| `tests/codec_error.rs` | `net_error_protocol_version_has_expected_and_got_fields` | ProtocolVersion struct variant display |
| `tests/codec_error.rs` | `net_error_peer_not_found_carries_label` | PeerNotFound(String) display |
| `tests/codec_error.rs` | `net_error_fingerprint_conflict_carries_description` | FingerprintConflict display |
| `tests/codec_error.rs` | `net_error_connection_closed_displays` | ConnectionClosed display |
| `tests/codec_error.rs` | `codec_round_trips_peer_message_hello` | encode/decode round-trip for Hello |
| `tests/codec_error.rs` | `codec_round_trips_peer_message_bye` | encode/decode round-trip for Bye |
| `tests/codec_error.rs` | `codec_max_frame_length_is_64k` | MAX_FRAME_LENGTH == 64*1024 |
| `tests/codec_error.rs` | `split_framed_returns_two_halves` | split_framed via real TCP pair (async) |
| `tests/event_connection.rs` | `peer_event_peer_pending_has_fingerprint_identicon_word_phrase` | PeerPending fields |
| `tests/event_connection.rs` | `peer_event_peer_connected_has_peer_id` | PeerConnected with PeerId |
| `tests/event_connection.rs` | `peer_event_peer_disconnected_has_peer_id` | PeerDisconnected with PeerId |
| `tests/event_connection.rs` | `connection_control_promote_trusted_is_debug` | ConnectionControl::PromoteTrusted |
| `tests/event_connection.rs` | `connection_control_reject_is_debug` | ConnectionControl::Reject |
| `tests/event_connection.rs` | `pending_peer_has_promote_tx_channel` | PendingPeer with mpsc::Sender |
| `tests/event_connection.rs` | `active_conn_has_peer_id` | ActiveConn struct |
| `tests/event_connection.rs` | `handshake_result_trusted_variant` | HandshakeResult::Trusted |
| `tests/event_connection.rs` | `handshake_result_pending_variant` | HandshakeResult::Pending |
| `tests/event_connection.rs` | `handshake_result_rejected_variant` | HandshakeResult::Rejected |
| `tests/event_connection.rs` | `default_port_is_7888` | DEFAULT_PORT constant |

**Final test counts:** periphore-net: 20/20 pass (9 codec_error + 11 event_connection); `cargo build --workspace` exits 0.

## Commits

| Hash | Message |
|------|---------|
| `c63b7ee` | test(06-02): add failing tests for NetError and codec (RED) |
| `bdfedb9` | feat(06-02): implement NetError enum and codec framing functions |
| `7188cb2` | test(06-02): add failing tests for PeerEvent and connection types (RED) |
| `a81a114` | feat(06-02): implement PeerEvent, connection types, and complete lib.rs exports |

## TDD Gate Compliance

Both tasks followed the RED/GREEN cycle:
1. RED: `test(06-02)` commits written with failing compilation errors (unresolved imports)
2. GREEN: `feat(06-02)` commits implement types to make all tests pass
3. REFACTOR: Not needed — code is minimal and clean as written

## Deviations from Plan

None — plan executed exactly as written.

## Threat Surface Scan

| Mitigated | File | Description |
|-----------|------|-------------|
| T-6-01 | codec.rs | `max_frame_length(64 * 1024)` in `split_framed()` — prevents OOM from malicious 4-byte length header claiming multi-GB frame |
| T-6-02 | connection.rs | `HandshakeResult::Pending` state enforced — unknown peers cannot reach `ActiveConn` without explicit `ConnectionControl::PromoteTrusted` from daemon |
| T-6-04 | error.rs | `NetError::FingerprintConflict` defined — Plan 03 connects this to `tracing::error!` + connection drop |

## Known Stubs

None — all types are data definitions. No data rendering, no async logic, no stub implementations.

## Self-Check: PASSED

- `crates/periphore-net/src/error.rs`: `pub enum NetError` with 8 variants including `ProtocolVersion { expected: u32, got: u32 }` and `PeerNotFound(String)` — FOUND
- `crates/periphore-net/src/codec.rs`: `pub fn split_framed`, `MAX_FRAME_LENGTH`, `max_frame_length(MAX_FRAME_LENGTH)`, CALLER RESPONSIBILITY comment — FOUND
- `crates/periphore-net/src/event.rs`: `pub enum PeerEvent` with `PeerPending { fingerprint: String, ... }` — FOUND
- `crates/periphore-net/src/connection.rs`: `pub enum HandshakeResult`, `PromoteTrusted`, `promote_tx: mpsc::Sender<ConnectionControl>` — FOUND
- `crates/periphore-net/src/lib.rs`: `mod error; pub mod codec; mod event; mod connection;`, `DEFAULT_PORT: u16 = 7888` — FOUND
- Commit `c63b7ee`: FOUND
- Commit `bdfedb9`: FOUND
- Commit `7188cb2`: FOUND
- Commit `a81a114`: FOUND
- `cargo build -p periphore-net`: exits 0
- `cargo build --workspace`: exits 0
- `cargo test --workspace`: all pass (20 new tests in periphore-net + all prior tests)
