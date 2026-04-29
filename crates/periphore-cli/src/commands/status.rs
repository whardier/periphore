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
pub(crate) async fn run(socket_path: &Path) -> anyhow::Result<()> {
    let response = ipc_request(socket_path, IpcRequest::GetStatus).await?;
    match response {
        IpcResponse::Status { running, fingerprint } => {
            println!("Daemon:      {}", if running { "running" } else { "not running" });
            match &fingerprint {
                Some(fp) => println!("Fingerprint: {fp}"),
                None     => println!("Fingerprint: (not available)"),
            }
            // Fetch and display identicon if the daemon is running.
            if running {
                let fp = fingerprint.as_deref().unwrap_or("").to_owned();
                match ipc_request(socket_path, IpcRequest::GetIdenticon { fingerprint: fp }).await {
                    Ok(IpcResponse::Identicon { identicon, .. }) if !identicon.is_empty() => {
                        print!("{identicon}");
                    }
                    _ => {}
                }
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
