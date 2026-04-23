use serde::{Deserialize, Serialize};

use crate::types::{Edge, EdgeMapping, MonitorInfo};

/// Wire protocol message type. All variants must serialize/deserialize via postcard.
/// Framing in production: tokio-util `LengthDelimitedCodec` (4-byte big-endian length header).
/// That framing lives in periphore-net (Phase 6); this crate owns the type surface only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PeerMessage {
    // -- Handshake --
    /// Initial greeting. `fingerprint` is a placeholder `[0u8; 32]` until Phase 2
    /// identity is implemented.
    Hello {
        protocol_version: u32,
        fingerprint:      [u8; 32],
        public_key:       Vec<u8>,
    },
    HelloAck {
        fingerprint: [u8; 32],
        public_key:  Vec<u8>,
        accepted:    bool,
    },

    // -- Topology --
    TopologyAdvertise {
        monitors: Vec<MonitorInfo>,
    },
    TopologyPropose {
        edges: Vec<EdgeMapping>,
    },
    TopologyAccept,
    TopologyReject {
        reason: String,
    },

    // -- Focus token --
    /// Transfers input focus to the peer. `entry_edge` is the edge the cursor crossed.
    /// `entry_position` is normalized 0.0..=1.0 along the edge.
    /// `sequence` is a monotonically increasing counter for deduplication.
    FocusTransfer {
        entry_edge:     Edge,
        entry_position: f64,
        sequence:       u64,
    },
    FocusAck {
        sequence: u64,
    },
    FocusReclaim,

    // -- Input events --
    /// Relative mouse movement in device units (not pixels; peer translates to its
    /// coordinate space).
    MouseMove {
        dx: i32,
        dy: i32,
    },
    MouseButton {
        button:  u8,
        pressed: bool,
    },
    /// Scroll deltas in device units. Both axes provided; peer uses what it needs.
    MouseScroll {
        dx: i32,
        dy: i32,
    },
    KeyEvent {
        scancode:  u32,
        pressed:   bool,
        modifiers: u8,
    },

    // -- Control --
    Ping {
        timestamp: u64,
    },
    Pong {
        timestamp: u64,
    },
    Bye,
}
