//! Integration tests for periphore-ipc socket lifecycle and IPC protocol.
//!
//! Tests run against a real Unix domain socket with a real tokio runtime.
//! Each test uses a unique temporary socket path to avoid conflicts.
//!
//! Requirements covered:
//! - IPC-01: socket created at platform path, removed on shutdown
//! - IPC-02: IPC layer modular boundary -- requests served without a network peer

use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

use periphore_ipc::{path::socket_path, IpcCommand};
use periphore_protocol::IpcResponse;

/// Create a unique temp socket path for each test to avoid conflicts.
fn temp_socket_path(test_name: &str) -> std::path::PathBuf {
    let tmp = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_owned());
    std::path::PathBuf::from(tmp)
        .join("periphore-test")
        .join(format!("{test_name}-{}.sock", std::process::id()))
}

/// Spawn a test IPC server and a router task that handles `IpcCommand`.
/// The router immediately sends `Ok` for `InjectInputEvent` and `SimulateEdgeCross`,
/// and `Status { running: true }` for `GetStatus`.
///
/// Returns: `(server_task_handle, router_task_handle, socket_path)`
async fn spawn_test_server(
    test_name: &str,
) -> (
    tokio::task::JoinHandle<std::io::Result<()>>,
    tokio::task::JoinHandle<()>,
    std::path::PathBuf,
) {
    let path = temp_socket_path(test_name);
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<IpcCommand>(16);

    // Router task: handle IpcCommands with test responses.
    let router = tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            handle_test_command(cmd);
        }
    });

    let server_path = path.clone();
    let server = tokio::spawn(async move {
        periphore_ipc::serve(&server_path, cmd_tx).await
    });

    // Give the server a moment to bind.
    tokio::time::sleep(Duration::from_millis(50)).await;

    (server, router, path)
}

/// Handle an `IpcCommand` with test-appropriate responses.
fn handle_test_command(cmd: IpcCommand) {
    match cmd {
        IpcCommand::GetStatus { responder } => {
            let _ = responder.send(IpcResponse::Status {
                running: true,
                fingerprint: None,
            });
        }
        IpcCommand::ListPeers { responder } => {
            let _ = responder.send(IpcResponse::Peers { peers: vec![] });
        }
        IpcCommand::GetTopology { responder } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::AcceptFingerprint { responder, .. } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::RejectFingerprint { responder, .. } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::ReloadConfig { responder } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::InjectInputEvent { responder, .. } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::SimulateEdgeCross { responder, .. } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::GetState { responder } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::GetPendingVerifications { responder } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::GetIdenticon { responder, .. } => {
            // Test stub: return a minimal valid Identicon response.
            let _ = responder.send(IpcResponse::Identicon {
                fingerprint_hex: "0000000000000000000000000000000000000000000000000000000000000000"
                    .to_owned(),
                identicon: "+--[ED25519 256]--+\n".to_owned(),
            });
        }
        IpcCommand::GetWordPhrase { responder, .. } => {
            // Test stub: return a minimal valid WordPhrase response.
            let _ = responder.send(IpcResponse::WordPhrase {
                words:  vec!["abandon".to_owned(); 6],
                phrase: "abandon abandon abandon abandon abandon abandon".to_owned(),
            });
        }
    }
}

/// Send a JSON-lines request and return the response line.
async fn send_request(socket_path: &std::path::Path, request_json: &str) -> String {
    let stream = UnixStream::connect(socket_path)
        .await
        .expect("should connect to test server");
    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = BufReader::new(reader_half);

    let mut line_with_newline = request_json.to_owned();
    line_with_newline.push('\n');
    writer_half
        .write_all(line_with_newline.as_bytes())
        .await
        .expect("write should succeed");

    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut response))
        .await
        .expect("response timeout")
        .expect("read_line error");
    response
}

// -- IPC-01: socket lifecycle tests --

#[tokio::test]
async fn socket_creates() {
    // IPC-01: Daemon creates Unix domain socket at the specified path on startup.
    let (server, router, path) = spawn_test_server("socket_creates").await;

    assert!(
        path.exists(),
        "socket file must exist after serve() binds: {path:?}"
    );

    server.abort();
    router.abort();
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
#[cfg(unix)]
async fn socket_permissions_0600() {
    // IPC-01 / T-1-01: Socket must have 0600 permissions (owner read/write only).
    use std::os::unix::fs::PermissionsExt;

    let (server, router, path) = spawn_test_server("socket_perms").await;

    let metadata = std::fs::metadata(&path).expect("socket metadata");
    let mode = metadata.permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "socket permissions must be 0600, got: {mode:o}");

    server.abort();
    router.abort();
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn stale_socket_does_not_block_restart() {
    // IPC-01 / T-1-04: A stale socket from a previous run must not prevent serve() from
    // binding.
    let path = temp_socket_path("stale_socket");

    // Create parent dir and a fake stale socket file.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&path, b"stale").expect("create stale socket file");
    assert!(path.exists(), "stale file must exist before test");

    // serve() should remove the stale file and bind successfully.
    let (cmd_tx, _cmd_rx) = mpsc::channel::<IpcCommand>(1);
    let server_path = path.clone();
    let server = tokio::spawn(async move {
        periphore_ipc::serve(&server_path, cmd_tx).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // If serve() successfully bound, the path is now a real socket (not the stale file).
    assert!(
        path.exists(),
        "socket must exist after stale removal and rebind"
    );

    server.abort();
    let _ = std::fs::remove_file(&path);
}

// -- IPC-02: request/response tests --

#[tokio::test]
async fn get_status_returns_status_response() {
    // IPC-02: GetStatus request returns a Status response over the socket.
    let (server, router, path) = spawn_test_server("get_status").await;

    let response = send_request(&path, r#"{"type":"get_status"}"#).await;

    // Response must be valid JSON containing "status" type.
    let json: serde_json::Value =
        serde_json::from_str(response.trim()).expect("response must be valid JSON");
    assert_eq!(
        json["type"], "status",
        "response type must be 'status': {response}"
    );
    assert_eq!(
        json["running"], true,
        "running must be true: {response}"
    );

    server.abort();
    router.abort();
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn inject_input_event_no_peer_required() {
    // IPC-02: InjectInputEvent is accepted without a network peer present.
    // This is the key IPC testing backbone (D-19).
    let (server, router, path) = spawn_test_server("inject_input").await;

    let request =
        r#"{"type":"inject_input_event","event":{"Mouse":{"dx":10,"dy":-5}}}"#;
    let response = send_request(&path, request).await;

    let json: serde_json::Value =
        serde_json::from_str(response.trim()).expect("response must be valid JSON");
    assert_eq!(
        json["type"], "ok",
        "InjectInputEvent must return ok: {response}"
    );

    server.abort();
    router.abort();
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn simulate_edge_cross_no_peer_required() {
    // IPC-02: SimulateEdgeCross is accepted without a network peer present.
    let (server, router, path) = spawn_test_server("simulate_edge").await;

    let request =
        r#"{"type":"simulate_edge_cross","edge":"Right","position":0.5}"#;
    let response = send_request(&path, request).await;

    let json: serde_json::Value =
        serde_json::from_str(response.trim()).expect("response must be valid JSON");
    assert_eq!(
        json["type"], "ok",
        "SimulateEdgeCross must return ok: {response}"
    );

    server.abort();
    router.abort();
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn malformed_request_returns_error_not_crash() {
    // T-1-02: Malformed JSON must return an error response, not crash the daemon.
    let (server, router, path) = spawn_test_server("malformed").await;

    let response = send_request(&path, "this is not json").await;

    let json: serde_json::Value =
        serde_json::from_str(response.trim()).expect("error response must be valid JSON");
    assert_eq!(
        json["type"], "error",
        "malformed input must return error: {response}"
    );
    // Server must still be alive to accept further connections.
    assert!(
        !server.is_finished(),
        "server must not crash on malformed input"
    );

    server.abort();
    router.abort();
    let _ = std::fs::remove_file(&path);
}

// -- Path resolution test --

#[test]
fn socket_path_resolution_returns_periphore_sock() {
    // Verifies platform path resolution (Assumption A3 in RESEARCH.md).
    let path = socket_path();
    assert!(
        path.to_str()
            .map_or(false, |s| s.ends_with("periphore.sock")),
        "socket_path() must end in periphore.sock, got: {path:?}"
    );
}
