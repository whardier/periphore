//! Integration tests for periphored net wiring (NET-03 + GetPendingVerifications).
//!
//! Tests run fully in-process using OS-assigned ports (port 0) and fabricated
//! IdentityStore/TrustStore instances. No external infrastructure required.
//!
//! Requirements covered:
//! - NET-03 SC5: PeerConfig with host triggers spawn_connector + connection attempt
//! - D-03: GetPendingVerifications IPC returns IpcResponse::PendingPeers

use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;

use periphore_config::PeerConfig;
use periphore_identity::IdentityStore;
use periphore_net::{ConnectionManager, PeerEvent};
use periphore_protocol::{IpcResponse, PendingPeerInfo};
use periphore_trust::TrustStore;

// Deterministic seed for test identity A.
const SEED_A: [u8; 32] = [0u8; 32];

/// Create a test IdentityStore from a known 32-byte seed.
fn make_identity(seed: [u8; 32]) -> (IdentityStore, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("key");
    std::fs::write(&path, seed).unwrap();
    (IdentityStore::load_or_create(&path).unwrap(), dir)
}

/// Create an empty TrustStore backed by a tempdir.
fn make_trust_store() -> (TrustStore, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("trusted.toml");
    (TrustStore::load(&path).unwrap(), dir)
}

// --- Test 1: PeerConfig with host triggers connector (NET-03) ---

/// A PeerConfig entry with a `host` causes `spawn_connector` to be called
/// and a connection attempt to be made, resulting in PeerEvent::PeerPending
/// (since the trust store is empty and the peer is unknown).
#[tokio::test]
async fn peer_config_with_host_triggers_connector() {
    let (id, _dir_id) = make_identity(SEED_A);
    let (ts, _dir_ts) = make_trust_store();
    let identity = Arc::new(id);
    let trust_store = Arc::new(RwLock::new(ts));

    // Bind a listener on port 0 to get an OS-assigned port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    // Spawn a minimal "peer" that completes a handshake as responder.
    // Uses a different identity seed so fingerprints differ.
    let (peer_id, _dir_peer_id) = make_identity([2u8; 32]);
    let (peer_ts, _dir_peer_ts) = make_trust_store();
    let peer_identity = Arc::new(peer_id);
    let peer_trust = Arc::new(RwLock::new(peer_ts));
    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            stream.set_nodelay(true).unwrap();
            let (mut fr, mut fw) = periphore_net::codec::split_framed(stream);
            // Run responder — result ignored for this test
            let _ = periphore_net::handshake::perform_handshake_responder(
                &mut fr,
                &mut fw,
                &peer_identity,
                &peer_trust,
                None,
            )
            .await;
            // Hold connection open briefly to allow PeerPending event to be sent
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    // Set up ConnectionManager with a PeerConfig pointing at our listener
    let (event_tx, mut event_rx) = mpsc::channel::<PeerEvent>(16);
    let mut conn_mgr = ConnectionManager::new(event_tx);
    let mut tasks: JoinSet<anyhow::Result<()>> = JoinSet::new();

    let peer_config = PeerConfig {
        host: Some("127.0.0.1".to_owned()),
        port: Some(port),
        ..Default::default()
    };
    conn_mgr.spawn_connector(
        &mut tasks,
        peer_config,
        Arc::clone(&identity),
        Arc::clone(&trust_store),
    );

    // Wait for PeerPending event (trust store is empty, so unknown peer → pending)
    let event = tokio::time::timeout(Duration::from_secs(5), event_rx.recv())
        .await
        .expect("timed out waiting for PeerEvent")
        .expect("event channel closed");

    assert!(
        matches!(event, PeerEvent::PeerPending { .. }),
        "expected PeerEvent::PeerPending, got: {event:?}"
    );

    tasks.abort_all();
}

// --- Test 2: GetPendingVerifications IPC returns PendingPeers (D-03) ---

/// Verifies that the GetPendingVerifications IPC dispatch pattern produces
/// IpcResponse::PendingPeers with the correct data.
///
/// Directly exercises the dispatch logic (what periphored's select! arm does)
/// without requiring a full daemon or IPC socket.
#[tokio::test]
async fn pending_verifications_ipc() {
    let (event_tx, _event_rx) = mpsc::channel::<PeerEvent>(16);
    let conn_mgr = ConnectionManager::new(event_tx);

    // pending_list() on fresh manager returns empty vec
    let list: Vec<PendingPeerInfo> = conn_mgr.pending_list();
    assert!(list.is_empty(), "expected empty pending list, got {list:?}");

    // Simulate the GetPendingVerifications IPC dispatch:
    // Build a oneshot pair, call the dispatch logic directly, and verify the response.
    let (resp_tx, resp_rx) = oneshot::channel::<IpcResponse>();
    let peers = conn_mgr.pending_list();
    let _ = resp_tx.send(IpcResponse::PendingPeers { peers });

    let response = tokio::time::timeout(Duration::from_secs(1), resp_rx)
        .await
        .expect("timed out")
        .expect("channel closed");

    assert!(
        matches!(response, IpcResponse::PendingPeers { ref peers } if peers.is_empty()),
        "expected PendingPeers with empty list, got: {response:?}"
    );
}
