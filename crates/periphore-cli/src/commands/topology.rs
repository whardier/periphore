//! Handler for `periphore topology`.
//!
//! Sends [`IpcRequest::GetTopology`] to the daemon. The daemon currently stubs
//! this as [`IpcResponse::Ok`] — Phase 8 adds the real topology variant. This
//! handler degrades gracefully with a clear message rather than treating `Ok`
//! as an error.

use std::path::Path;

use periphore_protocol::{IpcRequest, IpcResponse};

use crate::client::ipc_request;

/// Run the `topology` subcommand.
///
/// Prints the resolved monitor topology from the daemon. Until Phase 8 delivers
/// real topology data, prints a "not yet available" stub message.
///
/// # Errors
///
/// Returns an error if the daemon is not running or the IPC call fails.
pub async fn run(socket_path: &Path) -> anyhow::Result<()> {
    let response = ipc_request(socket_path, IpcRequest::GetTopology).await?;
    match response {
        // Phase 8 will add a real Topology variant to IpcResponse.
        // Until then, Ok is the daemon's correct stub response for GetTopology.
        IpcResponse::Ok => {
            println!("Topology: not yet available");
            println!("(Monitor topology is implemented in Phase 8)");
        }
        IpcResponse::Error { message } => {
            anyhow::bail!("daemon error: {message}");
        }
        other => {
            // Future-proof: when Phase 8 adds IpcResponse::Topology, update this arm.
            tracing::debug!(?other, "unexpected IPC response for GetTopology");
            anyhow::bail!("unexpected response from daemon");
        }
    }
    Ok(())
}
