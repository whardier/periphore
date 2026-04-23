use std::fs;
use std::path::Path;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

use periphore_protocol::{IpcRequest, IpcResponse};

use crate::IpcCommand;

/// Serve the IPC Unix domain socket.
///
/// Security mitigations applied here (RESEARCH.md security domain):
/// - T-1-01: Remove stale socket before bind; set 0600 permissions immediately after bind.
/// - T-1-04: Remove stale socket handles the "Denial of Service via stale socket file" threat.
/// - T-1-02: `handle_connection` never `.unwrap()` on IPC input; malformed lines log and skip.
pub async fn serve(
    socket_path: &Path,
    cmd_tx: mpsc::Sender<IpcCommand>,
) -> std::io::Result<()> {
    // T-1-04 + T-1-01: Remove stale socket from previous unclean shutdown.
    // .ok() suppresses the error if the file doesn't exist (normal case on first start).
    let _ = fs::remove_file(socket_path);

    // Ensure the parent directory exists (e.g., $TMPDIR/periphore/ may not exist on first run).
    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(socket_path)?;

    // T-1-01 + T-1-03 (Pitfall 3 in RESEARCH.md): Set socket permissions to 0600 immediately
    // after bind. UnixListener::bind respects umask; typical umask 0022 would leave 0644
    // (world-readable). Setting explicitly to 0600 means only the daemon owner can connect.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(socket_path, fs::Permissions::from_mode(0o600))?;
    }

    tracing::info!("IPC socket listening at {}", socket_path.display());

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let tx = cmd_tx.clone();
                tokio::spawn(handle_connection(stream, tx));
            }
            Err(e) => {
                tracing::error!("IPC accept error: {e}");
                // Continue serving; a single accept error should not crash the server.
            }
        }
    }
}

/// Handle a single IPC client connection.
///
/// Reads JSON-lines from the client. For each line:
/// - Parse as `IpcRequest` via `serde_json`.
/// - On success: send `IpcCommand` to daemon router; write `IpcResponse` back to client.
/// - On parse error (T-1-02): log warning, send error response -- never panic.
///
/// The connection is not authenticated. Authentication is handled at the OS level
/// via 0600 socket permissions restricting access to the daemon owner.
async fn handle_connection(stream: UnixStream, tx: mpsc::Sender<IpcCommand>) {
    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();

    while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        match serde_json::from_str::<IpcRequest>(trimmed) {
            Ok(req) => {
                // Create a one-shot channel for this request's response.
                let (resp_tx, resp_rx) = tokio::sync::oneshot::channel::<IpcResponse>();
                let cmd = IpcCommand::from_request_with_responder(req, resp_tx);

                if tx.send(cmd).await.is_err() {
                    // Daemon router has shut down; close connection.
                    tracing::warn!("IPC router channel closed; dropping client connection");
                    break;
                }

                // Wait for response from daemon router (with timeout to avoid hanging
                // clients).
                match tokio::time::timeout(std::time::Duration::from_secs(5), resp_rx).await
                {
                    Ok(Ok(response)) => {
                        let mut json = serde_json::to_string(&response).unwrap_or_else(
                            |_| {
                                r#"{"type":"error","message":"internal serialization error"}"#
                                    .to_owned()
                            },
                        );
                        json.push('\n');
                        if let Err(e) = writer_half.write_all(json.as_bytes()).await {
                            tracing::warn!("IPC write error: {e}");
                            break;
                        }
                    }
                    Ok(Err(_)) => {
                        // Responder dropped without sending -- send an error response.
                        let error_json =
                            r#"{"type":"error","message":"no response from daemon"}"#;
                        let _ = writer_half
                            .write_all(format!("{error_json}\n").as_bytes())
                            .await;
                    }
                    Err(_) => {
                        // Timeout -- send an error response and close connection.
                        let error_json =
                            r#"{"type":"error","message":"request timed out"}"#;
                        let _ = writer_half
                            .write_all(format!("{error_json}\n").as_bytes())
                            .await;
                        break;
                    }
                }
            }
            Err(e) => {
                // T-1-02: Never panic on bad IPC input. Log and skip.
                tracing::warn!(
                    "Malformed IPC request (ignored): {e}. Input: {trimmed:?}"
                );
                let error_json = format!(
                    r#"{{"type":"error","message":"malformed request: {}"}}"#,
                    e.to_string().replace('"', "'")
                );
                let _ = writer_half
                    .write_all(format!("{error_json}\n").as_bytes())
                    .await;
            }
        }
        line.clear();
    }
}
