//! periphore-net peer events: one-way notifications from ConnectionManager to periphored.
//!
//! Events flow from the network layer (periphore-net) to the daemon (periphored)
//! via an mpsc channel. The daemon handles each event in its select! loop.
//! There is no response from daemon back to the event sender — control in the
//! reverse direction uses ConnectionManager method calls or ConnectionControl channels.

use periphore_core::PeerId;

/// Events emitted by `ConnectionManager` and consumed by the daemon's select! loop.
#[derive(Debug)]
pub enum PeerEvent {
    /// An unknown peer completed the handshake and is held in pending state.
    /// The user must run `periphore trust accept <fingerprint>` to promote it.
    PeerPending {
        /// 64-char lowercase hex fingerprint of the pending peer.
        fingerprint: String,
        /// Pre-rendered Drunken Bishop identicon string (11 lines, newline-terminated).
        identicon: String,
        /// 6 BIP39 words for verbal verification.
        word_phrase: Vec<String>,
    },
    /// A peer completed the handshake and is trusted (either pre-trusted or just promoted).
    PeerConnected {
        /// Unique identity of the now-connected peer.
        peer_id: PeerId,
    },
    /// An established peer connection was dropped unexpectedly.
    /// The connector task will begin reconnecting with exponential backoff (D-09).
    PeerDisconnected {
        /// Unique identity of the disconnected peer.
        peer_id: PeerId,
    },
}
