//! periphore-ipc: Unix domain socket IPC service for Periphore.
//!
//! Provides:
//! - `serve()`: async IPC socket server (JSON-lines protocol)
//! - `path::socket_path()`: platform-correct socket path resolver
//! - `IpcCommand`: internal command type sent from IPC layer to daemon router
//!
//! Security: socket permissions 0600 (owner-only); stale socket removed before bind.
//! Protocol: JSON-lines (newline-delimited JSON) over Unix domain socket.
//! IPC owns transport; daemon owns routing (ARCHITECTURE.md Responsibility Map).

mod server;
pub mod path;

pub use server::serve;

use tokio::sync::oneshot;

use periphore_protocol::{Edge, InputEvent, IpcRequest, IpcResponse};

/// Internal command type sent from the IPC layer to the daemon's router via mpsc channel.
///
/// Each command carries a `responder` oneshot channel for the daemon to send the response
/// back through the IPC layer to the client. This design keeps routing in the daemon and
/// transport in the IPC crate (ARCHITECTURE.md Responsibility Map).
#[derive(Debug)]
pub enum IpcCommand {
    GetStatus {
        responder: oneshot::Sender<IpcResponse>,
    },
    ListPeers {
        responder: oneshot::Sender<IpcResponse>,
    },
    GetTopology {
        responder: oneshot::Sender<IpcResponse>,
    },
    AcceptFingerprint {
        fingerprint: String,
        responder: oneshot::Sender<IpcResponse>,
    },
    RejectFingerprint {
        fingerprint: String,
        responder: oneshot::Sender<IpcResponse>,
    },
    ReloadConfig {
        responder: oneshot::Sender<IpcResponse>,
    },
    InjectInputEvent {
        event: InputEvent,
        responder: oneshot::Sender<IpcResponse>,
    },
    SimulateEdgeCross {
        edge: Edge,
        position: f64,
        responder: oneshot::Sender<IpcResponse>,
    },
    GetState {
        responder: oneshot::Sender<IpcResponse>,
    },
    GetPendingVerifications {
        responder: oneshot::Sender<IpcResponse>,
    },
    GetDiscoveredPeers {
        responder: oneshot::Sender<IpcResponse>,
    },
    GetIdenticon {
        fingerprint: String,
        responder: oneshot::Sender<IpcResponse>,
    },
    GetWordPhrase {
        fingerprint: String,
        responder: oneshot::Sender<IpcResponse>,
    },
}

impl IpcCommand {
    /// Construct an `IpcCommand` from an `IpcRequest` and a response channel.
    pub fn from_request_with_responder(
        req: IpcRequest,
        responder: oneshot::Sender<IpcResponse>,
    ) -> Self {
        match req {
            IpcRequest::GetStatus => Self::GetStatus { responder },
            IpcRequest::ListPeers => Self::ListPeers { responder },
            IpcRequest::GetTopology => Self::GetTopology { responder },
            IpcRequest::AcceptFingerprint { fingerprint } => {
                Self::AcceptFingerprint {
                    fingerprint,
                    responder,
                }
            }
            IpcRequest::RejectFingerprint { fingerprint } => {
                Self::RejectFingerprint {
                    fingerprint,
                    responder,
                }
            }
            IpcRequest::ReloadConfig => Self::ReloadConfig { responder },
            IpcRequest::InjectInputEvent { event } => {
                Self::InjectInputEvent { event, responder }
            }
            IpcRequest::SimulateEdgeCross { edge, position } => {
                Self::SimulateEdgeCross {
                    edge,
                    position,
                    responder,
                }
            }
            IpcRequest::GetState => Self::GetState { responder },
            IpcRequest::GetPendingVerifications => {
                Self::GetPendingVerifications { responder }
            }
            IpcRequest::GetDiscoveredPeers => {
                Self::GetDiscoveredPeers { responder }
            }
            IpcRequest::GetIdenticon { fingerprint } => {
                Self::GetIdenticon {
                    fingerprint,
                    responder,
                }
            }
            IpcRequest::GetWordPhrase { fingerprint } => {
                Self::GetWordPhrase {
                    fingerprint,
                    responder,
                }
            }
        }
    }
}
