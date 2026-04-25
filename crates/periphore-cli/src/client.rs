//! IPC client transport for the `periphore` CLI.
//!
//! [`ipc_request`] connects to the daemon's Unix domain socket, sends one
//! [`IpcRequest`] as a JSON line, and returns the [`IpcResponse`].
//!
//! ENOENT and ECONNREFUSED are both mapped to a human-friendly "daemon not
//! running" error — see [`daemon_not_running_error`].

use std::path::Path;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use periphore_protocol::{IpcRequest, IpcResponse};

/// Send a single IPC request to the daemon and return its response.
///
/// # Errors
///
/// Returns a human-friendly error if the daemon is not running (ENOENT or
/// ECONNREFUSED), or an `anyhow` error for unexpected I/O or JSON failures.
pub async fn ipc_request(socket_path: &Path, req: IpcRequest) -> anyhow::Result<IpcResponse> {
    let stream = UnixStream::connect(socket_path)
        .await
        .map_err(|e| daemon_not_running_error(e, socket_path))?;

    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = BufReader::new(reader_half);

    let mut json = serde_json::to_string(&req)?;
    json.push('\n');
    writer_half.write_all(json.as_bytes()).await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response = serde_json::from_str::<IpcResponse>(line.trim())?;
    Ok(response)
}

/// Map a connection `io::Error` to a user-friendly anyhow error.
///
/// Both `NotFound` (socket file absent — daemon never started) and
/// `ConnectionRefused` (socket file present but no listener — daemon crashed)
/// map to the same "daemon is not running" message from the user's perspective.
fn daemon_not_running_error(e: std::io::Error, socket_path: &Path) -> anyhow::Error {
    use std::io::ErrorKind;
    match e.kind() {
        ErrorKind::NotFound => anyhow::anyhow!(
            "daemon is not running (socket not found: {})\nStart the daemon: periphored",
            socket_path.display()
        ),
        ErrorKind::ConnectionRefused => anyhow::anyhow!(
            "daemon is not running (connection refused: {})\nStart the daemon: periphored",
            socket_path.display()
        ),
        _ => anyhow::anyhow!("IPC connection failed: {e} ({})", socket_path.display()),
    }
}
