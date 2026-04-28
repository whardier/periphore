//! Handler for `periphore peers pending`.
//!
//! Sends [`IpcRequest::GetPendingVerifications`] to the daemon and displays peers
//! awaiting trust verification (D-11).

use std::path::Path;

use periphore_protocol::{IpcRequest, IpcResponse};

use crate::client::ipc_request;

/// Run the `peers pending` subcommand.
///
/// Connects to the daemon and prints all peers awaiting fingerprint verification.
///
/// # Errors
///
/// Returns an error if the daemon is not running or the IPC call fails.
pub(crate) async fn run(socket_path: &Path) -> anyhow::Result<()> {
    let response = ipc_request(socket_path, IpcRequest::GetPendingVerifications).await?;
    match response {
        IpcResponse::PendingPeers { peers } => {
            if peers.is_empty() {
                println!("No peers pending verification.");
                return Ok(());
            }
            for (i, peer) in peers.iter().enumerate() {
                if i > 0 {
                    println!();
                }
                println!("Peer {}:", i + 1);
                println!("  Fingerprint: {}", peer.fingerprint);
                println!("  Word phrase: {}", peer.word_phrase.join(" "));
                if !peer.identicon.is_empty() {
                    println!("  Identicon:");
                    for line in peer.identicon.lines() {
                        println!("    {line}");
                    }
                }
                println!();
                println!(
                    "  To trust: periphore trust accept {}",
                    peer.fingerprint
                );
            }
            println!();
            println!("{} peer(s) pending verification.", peers.len());
        }
        IpcResponse::Error { message } => {
            anyhow::bail!("daemon error: {message}");
        }
        other => {
            tracing::debug!(?other, "unexpected IPC response for GetPendingVerifications");
            anyhow::bail!("unexpected response from daemon");
        }
    }
    Ok(())
}
