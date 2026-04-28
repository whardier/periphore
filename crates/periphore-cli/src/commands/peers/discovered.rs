//! Handler for `periphore peers discovered`.
//!
//! Sends [`IpcRequest::GetDiscoveredPeers`] to the daemon and displays discovered
//! peers as a formatted table (D-10).

use std::path::Path;

use periphore_protocol::{IpcRequest, IpcResponse};

use crate::client::ipc_request;

/// Run the `peers discovered` subcommand.
///
/// Connects to the daemon and prints all peers discovered via mDNS or SSH tunnel probe.
///
/// # Errors
///
/// Returns an error if the daemon is not running or the IPC call fails.
pub(crate) async fn run(socket_path: &Path) -> anyhow::Result<()> {
    let response = ipc_request(socket_path, IpcRequest::GetDiscoveredPeers).await?;
    match response {
        IpcResponse::DiscoveredPeers { peers } => {
            if peers.is_empty() {
                println!("No peers discovered.");
                println!();
                println!("Make sure discovery is enabled in your config:");
                println!("  [discovery]");
                println!("  enabled = true");
                println!();
                println!("Both machines must be on the same subnet for mDNS discovery.");
                println!("For SSH tunnel peers, enable: ssh_probe_enabled = true");
                return Ok(());
            }
            // Table header
            println!(
                "{:<30} {:>5}  {:<10}  {}",
                "HOSTNAME", "PORT", "SOURCE", "LAST SEEN"
            );
            println!("{}", "-".repeat(70));
            for peer in &peers {
                // Format last_seen as human-readable relative time
                let age = format_age(peer.last_seen_epoch);
                println!(
                    "{:<30} {:>5}  {:<10}  {}",
                    peer.hostname, peer.port, peer.source, age
                );
            }
            println!();
            println!("{} peer(s) discovered.", peers.len());
        }
        IpcResponse::Error { message } => {
            anyhow::bail!("daemon error: {message}");
        }
        other => {
            tracing::debug!(?other, "unexpected IPC response for GetDiscoveredPeers");
            anyhow::bail!("unexpected response from daemon");
        }
    }
    Ok(())
}

/// Format a Unix epoch timestamp as a human-readable relative time string.
fn format_age(epoch_secs: u64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if epoch_secs > now {
        return "just now".to_owned();
    }
    let age_secs = now - epoch_secs;
    if age_secs < 60 {
        format!("{age_secs}s ago")
    } else if age_secs < 3600 {
        format!("{}m ago", age_secs / 60)
    } else {
        format!("{}h ago", age_secs / 3600)
    }
}
