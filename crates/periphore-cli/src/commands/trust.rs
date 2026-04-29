//! Handler for `periphore trust accept`.
//!
//! Sends [`IpcRequest::AcceptFingerprint`] to the daemon, which adds the
//! fingerprint to the trust cache and promotes any pending connection.

use std::path::Path;

use periphore_protocol::{IpcRequest, IpcResponse};

use crate::client::ipc_request;

/// Run `periphore trust accept <fingerprint>`.
///
/// Validates the fingerprint format, sends `AcceptFingerprint` to the daemon,
/// and reports the result.
///
/// # Errors
///
/// Returns an error if the fingerprint is malformed, the daemon is not running,
/// or the IPC call fails.
pub(crate) async fn run_accept(socket_path: &Path, fingerprint: &str) -> anyhow::Result<()> {
    // Basic format validation: must be exactly 64 lowercase hex characters.
    let fp = fingerprint.trim().to_ascii_lowercase();
    if fp.len() != 64 || !fp.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!(
            "invalid fingerprint: expected 64 hex characters, got {:?}\n\
             Copy the full fingerprint from the daemon log:\n\
             \x20 WARN unknown peer pending verification -- run: periphore trust accept <fingerprint>",
            fingerprint
        );
    }

    let response = ipc_request(socket_path, IpcRequest::AcceptFingerprint {
        fingerprint: fp.clone(),
    })
    .await?;

    match response {
        IpcResponse::Ok => {
            println!("trusted: {fp}");
            Ok(())
        }
        IpcResponse::Error { message } => {
            anyhow::bail!("daemon error: {message}");
        }
        other => {
            tracing::debug!(?other, "unexpected IPC response for AcceptFingerprint");
            anyhow::bail!("unexpected response from daemon");
        }
    }
}
