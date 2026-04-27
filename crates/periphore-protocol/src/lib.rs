//! periphore-protocol: shared wire protocol and IPC message types.
//!
//! All types are re-exported at the crate root for a clean import path:
//! `use periphore_protocol::{PeerMessage, IpcRequest, Edge, ...};`

pub mod ipc;
pub mod peer;
pub mod types;

// Re-export the most commonly used types at crate root
pub use ipc::{IpcRequest, IpcResponse, PendingPeerInfo};
pub use peer::PeerMessage;
pub use types::{Edge, EdgeMapping, InputEvent, KeyEventData, MonitorInfo, MouseEventData};
