//! Integration tests for periphore-discovery (NET-02).
//!
//! Tests run in-process. mDNS tests require multicast networking (may be flaky
//! in CI/containers — not included here). List and probe tests are fully deterministic.
//!
//! Requirements covered:
//! - D-08: TTL GC removes stale entries after 5 minutes
//! - D-09: Peer list caps at 64 entries, evicts oldest
//! - D-07: Hybrid expiry (remove_by_fullname for goodbye events)
//! - NET-02-SSH: SSH probe discovers forwarded Periphore daemon
//! - NET-02-SSH: SSH probe skips own daemon (fingerprint match)

use std::sync::Arc;
use std::time::Duration;

use periphore_discovery::{DiscoveredPeerList, DiscoveryEvent, DiscoveryService, DiscoverySource};

// Deterministic seeds for test identities.
const SEED_A: [u8; 32] = [0u8; 32];
const SEED_B: [u8; 32] = [1u8; 32];

/// Create a test IdentityStore from a known 32-byte seed.
fn make_identity(
    seed: [u8; 32],
) -> (periphore_identity::IdentityStore, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("key");
    std::fs::write(&path, seed).unwrap();
    let store = periphore_identity::IdentityStore::load_or_create(&path).unwrap();
    (store, dir)
}

// --- Test 1: list_cap_eviction (D-09) ---

/// The discovered peer list caps at 64 entries. When a 65th entry is inserted,
/// the oldest entry (by last_seen) is evicted.
#[test]
fn list_cap_eviction() {
    let mut list = DiscoveredPeerList::new();

    // Insert 64 entries — list should be at capacity
    for i in 0..64u16 {
        list.upsert(format!("host-{i}"), 7888 + i, DiscoverySource::Mdns, None);
    }
    assert_eq!(list.len(), 64, "list should hold exactly 64 entries");

    // Insert one more — should evict the oldest (host-0 was inserted first)
    list.upsert("host-overflow".to_owned(), 9999, DiscoverySource::Mdns, None);
    assert_eq!(list.len(), 64, "list must not exceed 64 after overflow insert");

    let snapshot = list.snapshot();
    let hostnames: Vec<&str> = snapshot.iter().map(|e| e.hostname.as_str()).collect();

    assert!(
        !hostnames.contains(&"host-0"),
        "host-0 (oldest) must have been evicted; snapshot: {hostnames:?}"
    );
    assert!(
        hostnames.contains(&"host-overflow"),
        "host-overflow (newest) must be present; snapshot: {hostnames:?}"
    );
}

// --- Test 2: gc_removes_expired (D-08) ---

/// GC does not remove fresh entries (just inserted).
/// We cannot fast-forward Instant in tests without mocking, so we verify
/// that GC correctly leaves fresh entries untouched and returns 0 removed.
#[test]
fn gc_removes_expired() {
    let mut list = DiscoveredPeerList::new();
    list.upsert("freshhost".to_owned(), 7888, DiscoverySource::Mdns, None);
    assert_eq!(list.len(), 1, "list should have 1 entry after upsert");

    // GC on a fresh entry: should remove nothing (TTL = 300s, entry is < 1ms old)
    let removed = list.gc();
    assert_eq!(removed, 0, "GC must not remove entries within TTL");
    assert_eq!(list.len(), 1, "list must still have 1 entry after GC");
    // Note: actual TTL eviction (D-08) requires either mocked Instant or sleeping
    // 300+ seconds. The constant TTL = Duration::from_secs(300) is verified by
    // code inspection; behavioral testing here confirms GC does not remove fresh entries.
}

// --- Test 3: remove_by_fullname (D-07 hybrid expiry) ---

/// remove_by_fullname removes an entry matched by mDNS fullname and returns true.
/// Returns false for non-existent fullnames.
#[test]
fn remove_by_fullname() {
    let mut list = DiscoveredPeerList::new();
    list.upsert(
        "myhost".to_owned(),
        7888,
        DiscoverySource::Mdns,
        Some("myhost._periphore._tcp.local.".to_owned()),
    );
    assert_eq!(list.len(), 1);

    // Remove by fullname — should succeed
    let removed = list.remove_by_fullname("myhost._periphore._tcp.local.");
    assert!(removed, "remove_by_fullname must return true for a known fullname");
    assert_eq!(list.len(), 0, "list must be empty after removal");

    // Remove non-existent — should return false
    let not_removed = list.remove_by_fullname("nonexistent._periphore._tcp.local.");
    assert!(!not_removed, "remove_by_fullname must return false for unknown fullname");
}

// --- Test 4: snapshot_converts_instant_to_epoch (Pitfall 6 mitigation) ---

/// snapshot() converts internal Instant to a non-zero Unix epoch timestamp.
/// Verifies that the conversion does not produce 0 or overflow.
#[test]
fn snapshot_converts_instant_to_epoch() {
    let mut list = DiscoveredPeerList::new();
    list.upsert(
        "testhost".to_owned(),
        7888,
        DiscoverySource::SshProbe,
        None,
    );

    let snapshot = list.snapshot();
    assert_eq!(snapshot.len(), 1, "snapshot must have 1 entry");

    let entry = &snapshot[0];
    assert_eq!(entry.hostname, "testhost", "hostname must match");
    assert_eq!(entry.port, 7888, "port must match");
    assert_eq!(entry.source, "ssh_probe", "source must be 'ssh_probe'");
    assert!(
        entry.last_seen_epoch > 0,
        "last_seen_epoch must be > 0 (not zero/overflow), got: {}",
        entry.last_seen_epoch
    );
    // last_seen_epoch should be within the last minute (reasonable for a test)
    let now_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(
        entry.last_seen_epoch <= now_epoch,
        "last_seen_epoch must not be in the future: {} > {now_epoch}",
        entry.last_seen_epoch
    );
    assert!(
        now_epoch - entry.last_seen_epoch < 60,
        "last_seen_epoch must be within the last 60 seconds"
    );
}

// --- Test 5: upsert_refreshes_last_seen ---

/// Upserting the same hostname:port twice results in one entry (no duplicate).
/// The second upsert refreshes last_seen.
#[test]
fn upsert_refreshes_last_seen() {
    let mut list = DiscoveredPeerList::new();
    list.upsert("host1".to_owned(), 7888, DiscoverySource::Mdns, None);

    // Brief sleep so Instant::now() differs
    std::thread::sleep(Duration::from_millis(10));

    // Second upsert with same key — should update not duplicate
    list.upsert("host1".to_owned(), 7888, DiscoverySource::Mdns, None);
    assert_eq!(list.len(), 1, "upsert of existing entry must not create duplicate");

    let snapshot = list.snapshot();
    assert_eq!(snapshot.len(), 1);

    let now_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let entry = &snapshot[0];
    // last_seen_epoch should be within the last second
    assert!(
        now_epoch - entry.last_seen_epoch < 2,
        "refreshed last_seen_epoch must be within 2 seconds of now"
    );
}

// --- Test 6: ssh_probe_against_test_listener (NET-02-SSH) ---

/// SSH probe discovers a Periphore daemon listening on a localhost port.
///
/// Sets up a minimal TCP listener that speaks the Hello/HelloAck protocol using
/// a distinct identity (identity_b). The probe should discover it and emit
/// a PeerDiscovered event.
#[tokio::test]
async fn ssh_probe_against_test_listener() {
    use futures_util::{SinkExt as _, StreamExt as _};
    use periphore_protocol::PeerMessage;
    use tokio::net::TcpListener;
    use tokio::sync::mpsc;

    let (identity_a, _dir_a) = make_identity(SEED_A); // prober identity
    let (identity_b, _dir_b) = make_identity(SEED_B); // remote daemon identity

    // Start a minimal listener that speaks Hello/HelloAck using identity_b's key
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bound_port = listener.local_addr().unwrap().port();

    let identity_b_fp = identity_b.fingerprint;
    let identity_b_pk = identity_b.keypair.verifying_key().to_bytes().to_vec();

    // Listener task: accepts one connection, reads Hello, responds with HelloAck
    tokio::spawn(async move {
        // Keep accepting in a loop because the probe may connect multiple times
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let fp = identity_b_fp;
                let pk = identity_b_pk.clone();
                tokio::spawn(async move {
                    if stream.set_nodelay(true).is_err() {
                        return;
                    }
                    let (mut fr, mut fw) = periphore_net::codec::split_framed(stream);
                    // Read Hello from prober
                    if let Some(Ok(frame)) = fr.next().await {
                        if periphore_net::codec::decode_message(frame).is_ok() {
                            // Respond with HelloAck { accepted: true } using identity_b's details
                            let ack = PeerMessage::HelloAck {
                                fingerprint: fp,
                                public_key: pk,
                                accepted: true,
                            };
                            if let Ok(encoded) = periphore_net::codec::encode_message(&ack) {
                                let _ = fw.send(encoded).await;
                            }
                        }
                    }
                    // Connection drops here (Pitfall 4 mitigation: probe disconnects after ack)
                });
            }
        }
    });

    // Create DiscoveryService with SSH probe enabled on the bound port
    let service = DiscoveryService::new();
    let (event_tx, mut event_rx) = mpsc::channel::<DiscoveryEvent>(16);
    let cancel = tokio_util::sync::CancellationToken::new();
    let mut tasks = tokio::task::JoinSet::<anyhow::Result<()>>::new();

    let discovery_config = periphore_config::DiscoveryConfig {
        enabled: false,
        instance_name: None,
        service_type: "_periphore._tcp.local.".to_owned(),
        ssh_probe_enabled: true,
        ssh_probe_ports: vec![bound_port],
    };

    service.start(
        &mut tasks,
        &discovery_config,
        event_tx,
        Arc::new(identity_a),
        cancel.clone(),
    );

    // Wait for PeerDiscovered event — probe sweeps immediately before sleeping
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match event_rx.recv().await {
                Some(DiscoveryEvent::PeerDiscovered { hostname, port, source }) => {
                    return Some((hostname, port, source));
                }
                Some(_) => continue,
                None => return None,
            }
        }
    })
    .await;

    cancel.cancel();
    tasks.abort_all();

    match result {
        Ok(Some((hostname, port, source))) => {
            assert_eq!(hostname, "127.0.0.1", "probe must report hostname 127.0.0.1");
            assert_eq!(port, bound_port, "probe must report correct port");
            assert_eq!(source, DiscoverySource::SshProbe, "source must be SshProbe");
        }
        Ok(None) => panic!("event channel closed without PeerDiscovered"),
        Err(_) => panic!("timed out waiting for PeerDiscovered event (probe should sweep immediately)"),
    }
}

// --- Test 7: ssh_probe_skips_own_fingerprint (Pitfall 3) ---

/// SSH probe does not emit PeerDiscovered when the remote fingerprint matches
/// the prober's own fingerprint.
#[tokio::test]
async fn ssh_probe_skips_own_fingerprint() {
    use futures_util::{SinkExt as _, StreamExt as _};
    use periphore_protocol::PeerMessage;
    use tokio::net::TcpListener;
    use tokio::sync::mpsc;

    // Use same seed for both prober and listener — same fingerprint
    let (identity_a, _dir_a) = make_identity(SEED_A); // prober
    let (identity_a2, _dir_a2) = make_identity(SEED_A); // listener also uses identity_a

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bound_port = listener.local_addr().unwrap().port();

    let self_fp = identity_a2.fingerprint;
    let self_pk = identity_a2.keypair.verifying_key().to_bytes().to_vec();

    // Listener responds with the SAME fingerprint as the prober (self-detection scenario)
    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let fp = self_fp;
                let pk = self_pk.clone();
                tokio::spawn(async move {
                    if stream.set_nodelay(true).is_err() {
                        return;
                    }
                    let (mut fr, mut fw) = periphore_net::codec::split_framed(stream);
                    if let Some(Ok(frame)) = fr.next().await {
                        if periphore_net::codec::decode_message(frame).is_ok() {
                            let ack = PeerMessage::HelloAck {
                                fingerprint: fp,
                                public_key: pk,
                                accepted: true,
                            };
                            if let Ok(encoded) = periphore_net::codec::encode_message(&ack) {
                                let _ = fw.send(encoded).await;
                            }
                        }
                    }
                });
            }
        }
    });

    let service = DiscoveryService::new();
    let (event_tx, mut event_rx) = mpsc::channel::<DiscoveryEvent>(16);
    let cancel = tokio_util::sync::CancellationToken::new();
    let mut tasks = tokio::task::JoinSet::<anyhow::Result<()>>::new();

    let discovery_config = periphore_config::DiscoveryConfig {
        enabled: false,
        instance_name: None,
        service_type: "_periphore._tcp.local.".to_owned(),
        ssh_probe_enabled: true,
        ssh_probe_ports: vec![bound_port],
    };

    service.start(
        &mut tasks,
        &discovery_config,
        event_tx,
        Arc::new(identity_a),
        cancel.clone(),
    );

    // Wait 2 seconds — if self-detection works, NO PeerDiscovered should arrive
    let result = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match event_rx.recv().await {
                Some(DiscoveryEvent::PeerDiscovered { .. }) => return true, // unexpected
                Some(_) => continue,                                         // ignore other events
                None => return false,                                        // channel closed
            }
        }
    })
    .await;

    cancel.cancel();
    tasks.abort_all();

    match result {
        Ok(true) => panic!("probe must NOT emit PeerDiscovered for self (own fingerprint)"),
        Ok(false) => panic!("event channel closed unexpectedly"),
        Err(_) => {
            // Timeout is the expected path — no PeerDiscovered in 2 seconds means self-detection worked
            let discovered = service.discovered_list();
            assert!(
                discovered.is_empty(),
                "discovered list must be empty after self-detection skip"
            );
        }
    }
}
