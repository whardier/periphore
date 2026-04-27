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

    /// Peer's fingerprint conflicts with the hard-configured expected fingerprint (SEC-06).
    #[error("fingerprint conflict: {0}")]
    FingerprintConflict(String),

    /// Protocol version mismatch (local vs remote).
    #[error("protocol version mismatch: local={expected}, remote={got}")]
    ProtocolVersion { expected: u32, got: u32 },

    /// promote_pending() called for a fingerprint not currently in pending state.
    #[error("peer not found in pending connections: {0}")]
    PeerNotFound(String),

    /// Internal error unrelated to the network protocol (e.g. lock poisoning).
    #[error("internal error: {0}")]
    Internal(String),
}
