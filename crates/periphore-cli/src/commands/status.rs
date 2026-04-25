//! Handler for `periphore status`.
//!
//! Sends [`IpcRequest::GetStatus`] to the daemon and prints the running state
//! and identity fingerprint to stdout.

use std::path::Path;

use periphore_protocol::{IpcRequest, IpcResponse};

use crate::client::ipc_request;

/// Run the `status` subcommand.
///
/// Connects to the daemon and prints whether it is running and its fingerprint.
///
/// # Errors
///
/// Returns an error if the daemon is not running or the IPC call fails.
pub async fn run(socket_path: &Path) -> anyhow::Result<()> {
    let response = ipc_request(socket_path, IpcRequest::GetStatus).await?;
    match response {
        IpcResponse::Status { running, fingerprint } => {
            println!("Daemon:      {}", if running { "running" } else { "not running" });
            match fingerprint {
                Some(fp) => println!("Fingerprint: {fp}"),
                None     => println!("Fingerprint: (not available)"),
            }
        }
        IpcResponse::Error { message } => {
            anyhow::bail!("daemon error: {message}");
        }
        other => {
            tracing::debug!(?other, "unexpected IPC response for GetStatus");
            anyhow::bail!("unexpected response from daemon");
        }
    }
    Ok(())
}
