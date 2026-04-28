use serde::{Deserialize, Serialize};

use crate::types::{Edge, InputEvent};

/// IPC request variants. All 12 variants per D-15.
/// Protocol: JSON-lines over Unix domain socket (newline-delimited JSON, D-16).
/// Tag field "type" uses `snake_case` variant names.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum IpcRequest {
    GetStatus,
    ListPeers,
    GetTopology,
    AcceptFingerprint {
        fingerprint: String,
    },
    RejectFingerprint {
        fingerprint: String,
    },
    ReloadConfig,
    /// Injects an input event locally. Key IPC testing primitive (D-19).
    InjectInputEvent {
        event: InputEvent,
    },
    /// Simulates an edge crossing locally. Key IPC testing primitive (D-19).
    SimulateEdgeCross {
        edge:     Edge,
        position: f64,
    },
    GetState,
    GetPendingVerifications,
    GetDiscoveredPeers,
    GetIdenticon {
        fingerprint: String,
    },
    GetWordPhrase {
        fingerprint: String,
    },
}

/// Information about a peer held in pending verification state.
/// Returned by GetPendingVerifications IPC command (D-03).
/// fingerprint: 64-char lowercase hex SHA-256 fingerprint.
/// identicon: pre-rendered Drunken Bishop terminal string (11 lines).
/// word_phrase: 6 BIP39 words for verbal verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPeerInfo {
    pub fingerprint:  String,
    pub identicon:    String,
    pub word_phrase:  Vec<String>,
}

/// Information about a peer discovered via mDNS or SSH tunnel probe.
/// Returned by GetDiscoveredPeers IPC command (Phase 7, D-06).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPeerInfo {
    /// Hostname or IP address of the discovered peer.
    pub hostname: String,
    /// TCP port the peer is listening on.
    pub port: u16,
    /// Seconds since Unix epoch when this peer was last seen.
    pub last_seen_epoch: u64,
    /// Discovery source: "mdns" or "ssh_probe".
    pub source: String,
}

/// IPC response variants. Extended in Phase 4.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum IpcResponse {
    Status {
        running:     bool,
        fingerprint: Option<String>,
    },
    Peers {
        peers: Vec<String>,
    },
    /// Response to GetPendingVerifications (Phase 6, D-03).
    /// peers: all connections awaiting user acceptance via AcceptFingerprint.
    PendingPeers {
        peers: Vec<PendingPeerInfo>,
    },
    /// Response to GetDiscoveredPeers (Phase 7, D-06).
    /// peers: all peers discovered via mDNS broadcast or SSH tunnel probe.
    DiscoveredPeers {
        peers: Vec<DiscoveredPeerInfo>,
    },
    /// Response to GetIdenticon (SEC-02, D-09).
    /// fingerprint_hex: 64-char lowercase hex of SHA-256 public key fingerprint.
    /// identicon: pre-rendered Drunken Bishop terminal string (11 lines, newline-terminated).
    Identicon {
        fingerprint_hex: String,
        identicon:       String,
    },
    /// Response to GetWordPhrase (SEC-03, D-10).
    /// words: 6 BIP39 words derived from fingerprint.
    /// phrase: space-joined convenience field (e.g., "abandon ability able about above absent").
    WordPhrase {
        words:  Vec<String>,
        phrase: String,
    },
    Ok,
    Error {
        message: String,
    },
}
