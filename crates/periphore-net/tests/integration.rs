//! Integration tests for periphore-net handshake protocol (NET-01).
//!
//! Tests run fully in-process using OS-assigned ports (port 0) and fabricated
//! IdentityStore/TrustStore instances. No external infrastructure required.
//!
//! Requirements covered:
//! - NET-01 SC1: trusted peer handshake completes with HandshakeResult::Trusted
//! - NET-01 SC1: unknown peer results in HandshakeResult::Pending with correct fingerprint
//! - T-6-03: protocol version mismatch returns Err(NetError::ProtocolVersion)
//! - SEC-06/D-04: fingerprint conflict returns Err(NetError::FingerprintConflict)
//! - D-02: promote_pending transitions pending peer to PeerConnected

use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};

use periphore_identity::IdentityStore;
use periphore_net::{
    codec, handshake,
    connection::HandshakeResult,
    NetError, PROTOCOL_VERSION,
};
use periphore_trust::TrustStore;

// Deterministic seeds for test identities — reproducible fingerprints.
const SEED_A: [u8; 32] = [0u8; 32];
const SEED_B: [u8; 32] = [1u8; 32];

/// Create a test IdentityStore from a known 32-byte seed.
/// The seed is written to a tempdir so `load_or_create` treats it as an existing key.
fn make_identity(seed: [u8; 32]) -> (IdentityStore, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("key");
    std::fs::write(&path, seed).unwrap();
    let store = IdentityStore::load_or_create(&path).unwrap();
    (store, dir)
}

/// Create an empty TrustStore backed by a tempdir.
/// The file does not need to exist — TrustStore::load returns empty cache.
fn make_trust_store() -> (TrustStore, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("trusted.toml");
    let store = TrustStore::load(&path).unwrap();
    (store, dir) // dir kept alive for path validity
}

/// Run a handshake pair in-process using OS-assigned port.
///
/// Binds a listener on port 0, connects a client, sets TCP_NODELAY on both sides,
/// and runs initiator + responder in parallel tasks.
/// Returns (initiator_result, responder_result).
async fn run_handshake_pair(
    initiator_identity: Arc<IdentityStore>,
    responder_identity: Arc<IdentityStore>,
    initiator_trust: Arc<RwLock<TrustStore>>,
    responder_trust: Arc<RwLock<TrustStore>>,
) -> (Result<HandshakeResult, NetError>, Result<HandshakeResult, NetError>) {
    run_handshake_pair_with_configs(
        initiator_identity,
        responder_identity,
        initiator_trust,
        responder_trust,
        None,
        None,
    )
    .await
}

/// Variant of run_handshake_pair that accepts optional PeerConfig for each side.
async fn run_handshake_pair_with_configs(
    initiator_identity: Arc<IdentityStore>,
    responder_identity: Arc<IdentityStore>,
    initiator_trust: Arc<RwLock<TrustStore>>,
    responder_trust: Arc<RwLock<TrustStore>>,
    initiator_config: Option<periphore_config::PeerConfig>,
    responder_config: Option<periphore_config::PeerConfig>,
) -> (Result<HandshakeResult, NetError>, Result<HandshakeResult, NetError>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    let (init_tx, init_rx) = tokio::sync::oneshot::channel();

    // Responder task
    let r_id = Arc::clone(&responder_identity);
    let r_ts = Arc::clone(&responder_trust);
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        stream.set_nodelay(true).unwrap();
        let (mut fr, mut fw) = codec::split_framed(stream);
        let result = handshake::perform_handshake_responder(
            &mut fr,
            &mut fw,
            &r_id,
            &r_ts,
            responder_config.as_ref(),
        )
        .await;
        let _ = resp_tx.send(result);
    });

    // Initiator task
    let i_id = Arc::clone(&initiator_identity);
    let i_ts = Arc::clone(&initiator_trust);
    tokio::spawn(async move {
        let stream = TcpStream::connect(addr).await.unwrap();
        stream.set_nodelay(true).unwrap();
        let (mut fr, mut fw) = codec::split_framed(stream);
        let result = handshake::perform_handshake_initiator(
            &mut fr,
            &mut fw,
            &i_id,
            &i_ts,
            initiator_config.as_ref(),
        )
        .await;
        let _ = init_tx.send(result);
    });

    let timeout_dur = Duration::from_secs(5);
    let init_result = tokio::time::timeout(timeout_dur, init_rx)
        .await
        .expect("initiator timed out")
        .expect("initiator channel closed");
    let resp_result = tokio::time::timeout(timeout_dur, resp_rx)
        .await
        .expect("responder timed out")
        .expect("responder channel closed");

    (init_result, resp_result)
}

// --- Test 1: Trusted peer handshake ---

/// Both peers have each other's fingerprint in their trust stores.
/// Both should complete handshake with HandshakeResult::Trusted.
#[tokio::test]
async fn handshake_trusted_peer() {
    let (id_a, _dir_a) = make_identity(SEED_A);
    let (id_b, _dir_b) = make_identity(SEED_B);
    let (mut ts_a, dir_ts_a) = make_trust_store();
    let (mut ts_b, dir_ts_b) = make_trust_store();

    // A trusts B, B trusts A
    let path_a = dir_ts_a.path().join("trusted.toml");
    let path_b = dir_ts_b.path().join("trusted.toml");
    ts_a.add_trusted(&id_b.fingerprint_hex(), None, &path_a).unwrap();
    ts_b.add_trusted(&id_a.fingerprint_hex(), None, &path_b).unwrap();

    let ts_a = Arc::new(RwLock::new(ts_a));
    let ts_b = Arc::new(RwLock::new(ts_b));
    let id_a = Arc::new(id_a);
    let id_b = Arc::new(id_b);

    let (init_result, resp_result) = run_handshake_pair(
        Arc::clone(&id_a),
        Arc::clone(&id_b),
        Arc::clone(&ts_a),
        Arc::clone(&ts_b),
    )
    .await;

    assert!(
        matches!(init_result, Ok(HandshakeResult::Trusted { .. })),
        "initiator should be Trusted, got: {init_result:?}"
    );
    assert!(
        matches!(resp_result, Ok(HandshakeResult::Trusted { .. })),
        "responder should be Trusted, got: {resp_result:?}"
    );
}

// --- Test 2: Unknown peer goes pending ---

/// Empty trust stores on both sides.
/// Both peers should see the other as Pending with the correct fingerprint.
#[tokio::test]
async fn handshake_unknown_peer_goes_pending() {
    let (id_a, _dir_a) = make_identity(SEED_A);
    let (id_b, _dir_b) = make_identity(SEED_B);
    let (ts_a, _dir_ts_a) = make_trust_store();
    let (ts_b, _dir_ts_b) = make_trust_store();
    let ts_a = Arc::new(RwLock::new(ts_a));
    let ts_b = Arc::new(RwLock::new(ts_b));
    let fp_b = id_b.fingerprint_hex();
    let id_a = Arc::new(id_a);
    let id_b = Arc::new(id_b);

    let (init_result, resp_result) = run_handshake_pair(
        Arc::clone(&id_a),
        Arc::clone(&id_b),
        Arc::clone(&ts_a),
        Arc::clone(&ts_b),
    )
    .await;

    // Both sides should see the other as Pending
    assert!(
        matches!(init_result, Ok(HandshakeResult::Pending { .. })),
        "initiator should be Pending, got: {init_result:?}"
    );
    assert!(
        matches!(resp_result, Ok(HandshakeResult::Pending { .. })),
        "responder should be Pending, got: {resp_result:?}"
    );

    // The initiator's pending fingerprint should be peer B's fingerprint
    if let Ok(HandshakeResult::Pending { fingerprint_hex, .. }) = &init_result {
        assert_eq!(
            *fingerprint_hex, fp_b,
            "initiator's pending fingerprint should be peer B's fingerprint"
        );
    }
}

// --- Test 3: Protocol version mismatch ---

/// The responder receives a Hello with protocol_version=99.
/// The responder should return Err(NetError::ProtocolVersion { got: 99, .. }).
#[tokio::test]
async fn protocol_version_mismatch() {
    use bytes::BytesMut;
    use futures_util::{SinkExt as _, StreamExt as _};
    use periphore_protocol::PeerMessage;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (id_a, _dir_a) = make_identity(SEED_A);
    let (ts_a, _dir_ts_a) = make_trust_store();
    let ts_a = Arc::new(RwLock::new(ts_a));
    let id_a = Arc::new(id_a);

    // Responder: runs the normal responder
    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
    let r_id = Arc::clone(&id_a);
    let r_ts = Arc::clone(&ts_a);
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        stream.set_nodelay(true).unwrap();
        let (mut fr, mut fw) = codec::split_framed(stream);
        let result =
            handshake::perform_handshake_responder(&mut fr, &mut fw, &r_id, &r_ts, None).await;
        let _ = result_tx.send(result);
    });

    // "Bad" initiator: sends Hello with wrong protocol_version = 99
    let (fake_init_id, _dir_b) = make_identity(SEED_B);
    tokio::spawn(async move {
        let stream = TcpStream::connect(addr).await.unwrap();
        stream.set_nodelay(true).unwrap();
        let (mut fr, mut fw) = codec::split_framed(stream);
        let bad_hello = PeerMessage::Hello {
            protocol_version: 99, // Wrong version — triggers T-6-03 rejection
            fingerprint: fake_init_id.fingerprint,
            public_key: fake_init_id.keypair.verifying_key().to_bytes().to_vec(),
        };
        let encoded = codec::encode_message(&bad_hello).unwrap();
        fw.send(encoded).await.unwrap();
        // Read the HelloAck { accepted: false } back
        if let Some(Ok(frame)) = fr.next().await {
            let frame = BytesMut::from(frame.as_ref());
            if let Ok(PeerMessage::HelloAck { accepted, .. }) = codec::decode_message(frame) {
                assert!(
                    !accepted,
                    "HelloAck should have accepted=false on version mismatch"
                );
            }
        }
    });

    let responder_result = tokio::time::timeout(Duration::from_secs(5), result_rx)
        .await
        .expect("timed out")
        .expect("channel closed");

    assert!(
        matches!(responder_result, Err(NetError::ProtocolVersion { got: 99, .. })),
        "responder should return ProtocolVersion error, got: {responder_result:?}"
    );
}

// --- Test 4: Codec round-trip (non-async) ---

/// encode_message followed by decode_message must produce the identical message.
#[test]
fn codec_roundtrip_hello() {
    use bytes::BytesMut;
    use periphore_protocol::PeerMessage;

    let msg = PeerMessage::Hello {
        protocol_version: PROTOCOL_VERSION,
        fingerprint: [42u8; 32],
        public_key: vec![1, 2, 3, 4],
    };
    let encoded = codec::encode_message(&msg).expect("encode");
    let frame = BytesMut::from(encoded.as_ref());
    let decoded = codec::decode_message(frame).expect("decode");
    assert_eq!(msg, decoded, "round-trip must be identity");
}

// --- Test 5: Fingerprint conflict ---

/// Initiator has a PeerConfig with a configured fingerprint that does NOT match
/// the actual responder fingerprint. The initiator must return Err(NetError::FingerprintConflict).
#[tokio::test]
async fn fingerprint_conflict() {
    use periphore_config::PeerConfig;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (id_a, _dir_a) = make_identity(SEED_A); // initiator identity
    let (id_b, _dir_b) = make_identity(SEED_B); // responder identity
    let (ts_a, _dir_ts_a) = make_trust_store();
    let (ts_b, _dir_ts_b) = make_trust_store();
    let ts_a = Arc::new(RwLock::new(ts_a));
    let ts_b = Arc::new(RwLock::new(ts_b));
    let id_a = Arc::new(id_a);
    let id_b = Arc::new(id_b);

    // Responder: no peer_config (no configured fingerprint)
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    let r_id = Arc::clone(&id_b);
    let r_ts = Arc::clone(&ts_b);
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        stream.set_nodelay(true).unwrap();
        let (mut fr, mut fw) = codec::split_framed(stream);
        let result =
            handshake::perform_handshake_responder(&mut fr, &mut fw, &r_id, &r_ts, None).await;
        let _ = resp_tx.send(result);
    });

    // Initiator: PeerConfig with a fingerprint that does NOT match id_b's actual fingerprint.
    // Use all-zeros hex string — guaranteed not to match any real identity.
    let wrong_fingerprint =
        "0000000000000000000000000000000000000000000000000000000000000000".to_owned();
    let peer_config = PeerConfig {
        fingerprint: Some(wrong_fingerprint),
        host: Some("127.0.0.1".to_owned()),
        port: Some(addr.port()),
        ..Default::default()
    };
    let i_id = Arc::clone(&id_a);
    let i_ts = Arc::clone(&ts_a);
    let (init_tx, init_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let stream = TcpStream::connect(addr).await.unwrap();
        stream.set_nodelay(true).unwrap();
        let (mut fr, mut fw) = codec::split_framed(stream);
        let result = handshake::perform_handshake_initiator(
            &mut fr,
            &mut fw,
            &i_id,
            &i_ts,
            Some(&peer_config),
        )
        .await;
        let _ = init_tx.send(result);
    });

    let init_result = tokio::time::timeout(Duration::from_secs(5), init_rx)
        .await
        .expect("timed out")
        .expect("channel closed");

    // The initiator must return FingerprintConflict — not Ok(Trusted) or Ok(Pending)
    assert!(
        matches!(init_result, Err(NetError::FingerprintConflict(_))),
        "initiator with wrong configured fingerprint should return FingerprintConflict, got: {init_result:?}"
    );

    // Responder may see the connection close; drain it to avoid task leak warnings.
    let _ = tokio::time::timeout(Duration::from_secs(2), resp_rx).await;
}

// --- Test 6: promote_pending ---

/// Full pending→promoted→connected flow using ConnectionManager.
/// Verifies that after promote_pending() is called, PeerEvent::PeerConnected arrives.
#[tokio::test]
async fn promote_pending() {
    use periphore_net::{ConnectionManager, PeerEvent};
    use tokio::task::JoinSet;

    // A is the daemon running ConnectionManager, B is the connecting peer
    let (id_a, _dir_a) = make_identity(SEED_A);
    let (id_b, _dir_b) = make_identity(SEED_B);
    let (ts_a, _dir_ts_a) = make_trust_store();
    let (ts_b, _dir_ts_b) = make_trust_store();
    let id_a = Arc::new(id_a);
    let id_b = Arc::new(id_b);
    let ts_a = Arc::new(RwLock::new(ts_a));
    let ts_b = Arc::new(RwLock::new(ts_b));

    // Set up ConnectionManager with its own listener
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<PeerEvent>(16);
    let mut conn_mgr = ConnectionManager::new(event_tx);
    let mut tasks: JoinSet<anyhow::Result<()>> = JoinSet::new();

    // Bind on port 0, then pass the SocketAddr to spawn_listener.
    // spawn_listener will bind its own listener on that address.
    // We pre-bind to discover the port, then drop our listener so spawn_listener can rebind.
    let tmp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bound_addr = tmp_listener.local_addr().unwrap();
    drop(tmp_listener); // Release — TOCTOU window acceptable in tests

    conn_mgr.spawn_listener(
        &mut tasks,
        bound_addr,
        Arc::clone(&id_a),
        Arc::clone(&ts_a),
    );

    // Small delay for spawn_listener to bind
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Spawn peer B as the outbound connector — no trust store entry for A on B's side either
    let id_b_clone = Arc::clone(&id_b);
    let ts_b_clone = Arc::clone(&ts_b);
    tokio::spawn(async move {
        let stream = TcpStream::connect(bound_addr).await.unwrap();
        stream.set_nodelay(true).unwrap();
        let (mut fr, mut fw) = codec::split_framed(stream);
        // B is the initiator — runs handshake from B's perspective (B doesn't trust A either)
        let _ = handshake::perform_handshake_initiator(
            &mut fr,
            &mut fw,
            &id_b_clone,
            &ts_b_clone,
            None,
        )
        .await;
        // Hold connection open long enough for promote_pending to complete
        tokio::time::sleep(Duration::from_secs(3)).await;
    });

    // Wait for PeerPending event — fingerprint is peer B's
    let pending_event = tokio::time::timeout(Duration::from_secs(5), event_rx.recv())
        .await
        .expect("timed out waiting for PeerPending")
        .expect("channel closed");

    let peer_fp = match &pending_event {
        PeerEvent::PeerPending { fingerprint, .. } => fingerprint.clone(),
        other => panic!("expected PeerPending, got: {other:?}"),
    };

    // Now promote the pending peer
    conn_mgr
        .promote_pending(&peer_fp)
        .await
        .expect("promote_pending should succeed");

    // Assert PeerConnected arrives after promotion
    let connected_event = tokio::time::timeout(Duration::from_secs(5), event_rx.recv())
        .await
        .expect("timed out waiting for PeerConnected")
        .expect("channel closed");

    assert!(
        matches!(connected_event, PeerEvent::PeerConnected { .. }),
        "expected PeerConnected after promote_pending, got: {connected_event:?}"
    );

    tasks.abort_all();
}
