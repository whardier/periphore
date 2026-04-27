//! periphore-net: TCP peer connections, handshake, and connection lifecycle.
//!
//! Provides:
//! - `ConnectionManager`: accepts inbound and initiates outbound peer connections
//! - `PeerEvent`: one-way notifications from the network layer to the daemon
//! - `NetError`: typed error enum for all network operations
//! - `DEFAULT_PORT`: default TCP listen port (7888)
//!
//! D-18: Framing uses LengthDelimitedCodec (4-byte big-endian) + postcard.
//! D-19: TCP_NODELAY is set immediately after every connect() and accept() — hard requirement.

mod error;
pub mod codec;
mod event;
mod connection;
mod handshake;
// manager is added in Plan 03 (Task 2).

pub use error::NetError;
pub use event::PeerEvent;
pub use connection::{ActiveConn, ConnectionControl, HandshakeResult, PendingPeer};
pub use codec::MAX_FRAME_LENGTH;
pub use handshake::PROTOCOL_VERSION;

/// Default TCP port for peer connections (IANA unassigned, D-08).
pub const DEFAULT_PORT: u16 = 7888;
