//! Integration tests for the periphore CLI IPC client.
//!
//! Uses a mock IPC server (real Unix socket, real periphore_ipc::serve, mock command
//! router) to test the CLI client path without a running periphored daemon.
//!
//! Requirements covered:
//! - SC1: periphore status connects and prints running + fingerprint
//! - TOP-04: periphore topology shows stub message (IpcResponse::Ok from daemon)
//! - SC3: periphore fails gracefully when daemon is not running

use std::time::Duration;

use tokio::sync::mpsc;

use periphore_cli::client::ipc_request;
use periphore_ipc::IpcCommand;
use periphore_protocol::{IpcRequest, IpcResponse};

/// Create a unique temp socket path for each test to avoid conflicts.
fn temp_socket_path(test_name: &str) -> std::path::PathBuf {
    let tmp = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_owned());
    std::path::PathBuf::from(tmp)
        .join("periphore-test")
        .join(format!("cli-{test_name}-{}.sock", std::process::id()))
}

/// Spawn a mock IPC server with a test command router.
///
/// Returns `(server_handle, router_handle, socket_path)`. Teardown:
/// `server.abort(); router.abort(); std::fs::remove_file(&path)`.
async fn spawn_test_server(
    test_name: &str,
) -> (
    tokio::task::JoinHandle<std::io::Result<()>>,
    tokio::task::JoinHandle<()>,
    std::path::PathBuf,
) {
    let path = temp_socket_path(test_name);
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<IpcCommand>(16);

    let router = tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            handle_test_command(cmd);
        }
    });

    let server_path = path.clone();
    let server = tokio::spawn(async move { periphore_ipc::serve(&server_path, cmd_tx).await });

    // Give the server a moment to bind.
    tokio::time::sleep(Duration::from_millis(50)).await;

    (server, router, path)
}

/// Handle IpcCommands with test-appropriate responses.
fn handle_test_command(cmd: IpcCommand) {
    match cmd {
        IpcCommand::GetStatus { responder } => {
            let _ = responder.send(IpcResponse::Status {
                running:     true,
                fingerprint: Some("abcd1234efgh5678".to_owned()),
            });
        }
        IpcCommand::GetTopology { responder } => {
            // Daemon stub: returns Ok until Phase 8 adds real topology data.
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::ListPeers { responder } => {
            let _ = responder.send(IpcResponse::Peers { peers: vec![] });
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
            let _ = responder.send(IpcResponse::PendingPeers { peers: vec![] });
        }
        IpcCommand::GetDiscoveredPeers { responder } => {
            let _ = responder.send(IpcResponse::DiscoveredPeers { peers: vec![] });
        }
        IpcCommand::GetIdenticon { responder, .. } => {
            let _ = responder.send(IpcResponse::Identicon {
                fingerprint_hex: "0000000000000000000000000000000000000000000000000000000000000000"
                    .to_owned(),
                identicon: "+--[ED25519 256]--+\n".to_owned(),
            });
        }
        IpcCommand::GetWordPhrase { responder, .. } => {
            let _ = responder.send(IpcResponse::WordPhrase {
                words:  vec!["abandon".to_owned(); 6],
                phrase: "abandon abandon abandon abandon abandon abandon".to_owned(),
            });
        }
    }
}

// -- SC1: status command --

#[tokio::test]
async fn status_command_prints_running_and_fingerprint() {
    // SC1: ipc_request returns IpcResponse::Status with running=true and fingerprint set.
    let (server, router, path) = spawn_test_server("status_running").await;

    let response = ipc_request(&path, IpcRequest::GetStatus)
        .await
        .expect("ipc_request must succeed against mock server");

    match response {
        IpcResponse::Status { running, fingerprint } => {
            assert!(running, "daemon must report running=true");
            assert!(
                fingerprint.is_some(),
                "daemon must return a fingerprint for the status response"
            );
        }
        other => panic!("expected Status response, got: {other:?}"),
    }

    server.abort();
    router.abort();
    let _ = std::fs::remove_file(&path);
}

// -- TOP-04: topology command --

#[tokio::test]
async fn topology_command_receives_ok_stub_without_error() {
    // TOP-04: ipc_request returns IpcResponse::Ok for GetTopology (daemon stub until Phase 8).
    // The client must NOT treat Ok as an error for this command.
    let (server, router, path) = spawn_test_server("topology_stub").await;

    let response = ipc_request(&path, IpcRequest::GetTopology)
        .await
        .expect("ipc_request must succeed against mock server");

    assert!(
        matches!(response, IpcResponse::Ok),
        "topology command must receive IpcResponse::Ok from daemon stub, got: {response:?}"
    );

    server.abort();
    router.abort();
    let _ = std::fs::remove_file(&path);
}

// -- SC3: daemon not running --

#[tokio::test]
async fn status_fails_gracefully_when_daemon_not_running() {
    // SC3: ipc_request returns Err with "daemon is not running" message when
    // the socket file does not exist (ENOENT — clean daemon-never-started case).
    let path = temp_socket_path("no_daemon");
    // Do NOT spawn a server — we want the connect to fail with ENOENT.

    let result = ipc_request(&path, IpcRequest::GetStatus).await;

    assert!(result.is_err(), "ipc_request must fail when daemon is not running");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("daemon is not running"),
        "error message must contain 'daemon is not running', got: {err_msg}"
    );
}
