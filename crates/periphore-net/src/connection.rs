//! periphore-net connection state types.
//!
//! PendingPeer: connection that completed handshake with unknown fingerprint.
//! ActiveConn: trusted connection (input forwarding path, Phase 9).
//! ConnectionControl: daemon→peer control messages over mpsc channel.
//! HandshakeResult: outcome of perform_handshake_* functions.

use tokio::sync::mpsc;

use periphore_core::PeerId;

/// Control messages sent from the daemon to a pending connection task.
#[derive(Debug)]
pub enum ConnectionControl {
    /// User accepted the fingerprint — promote this connection to active.
    PromoteTrusted,
    /// User rejected — close the connection.
    Reject,
}

/// A peer connection that completed the handshake with an unrecognized fingerprint.
/// Held pending until the user accepts or rejects via IPC.
#[derive(Debug)]
pub struct PendingPeer {
    /// 64-char lowercase hex fingerprint.
    pub fingerprint_hex: String,
    /// Pre-rendered Drunken Bishop identicon (11 lines).
    pub identicon: String,
    /// 6 BIP39 words for verbal verification.
    pub word_phrase: Vec<String>,
    /// Send PromoteTrusted or Reject to the connection task.
    pub promote_tx: mpsc::Sender<ConnectionControl>,
}

/// A fully trusted and active peer connection.
/// Phase 9 adds the input forwarding channel here.
/// Phase 6 only tracks that the peer is connected.
#[derive(Debug)]
pub struct ActiveConn {
    /// Unique identity of the peer.
    pub peer_id: PeerId,
}

/// Outcome of a handshake attempt.
#[derive(Debug)]
pub enum HandshakeResult {
    /// The peer's fingerprint was found in the trust store — connection is trusted.
    Trusted {
        peer_id: PeerId,
        /// 64-char lowercase hex fingerprint (for ActiveConn tracking).
        fingerprint_hex: String,
    },
    /// The peer's fingerprint was not found in the trust store — requires user acceptance.
    Pending {
        peer_id: PeerId,
        fingerprint_hex: String,
        identicon: String,
        word_phrase: Vec<String>,
    },
    /// The peer rejected our identity, or protocol version mismatch, or fingerprint conflict.
    Rejected {
        reason: String,
    },
}
