/// TDD tests for periphore-net event.rs and connection.rs (Plan 06-02, Task 2).
///
/// RED phase: these tests are written before the implementation exists.
/// They define the exact contract that event.rs and connection.rs must satisfy.

// --- PeerEvent tests ---

#[test]
fn peer_event_peer_pending_has_fingerprint_identicon_word_phrase() {
    use periphore_net::PeerEvent;
    let event = PeerEvent::PeerPending {
        fingerprint: "aabbcc".to_string(),
        identicon: "icon".to_string(),
        word_phrase: vec!["alpha".to_string(), "beta".to_string()],
    };
    // Debug must work (derive(Debug))
    let dbg = format!("{event:?}");
    assert!(dbg.contains("PeerPending"), "expected 'PeerPending' in: {dbg}");
    assert!(dbg.contains("aabbcc"), "expected fingerprint in: {dbg}");
}

#[test]
fn peer_event_peer_connected_has_peer_id() {
    use periphore_core::PeerId;
    use periphore_net::PeerEvent;
    let event = PeerEvent::PeerConnected {
        peer_id: PeerId::new("deadbeef"),
    };
    let dbg = format!("{event:?}");
    assert!(dbg.contains("PeerConnected"), "expected 'PeerConnected' in: {dbg}");
    assert!(dbg.contains("deadbeef"), "expected peer_id in: {dbg}");
}

#[test]
fn peer_event_peer_disconnected_has_peer_id() {
    use periphore_core::PeerId;
    use periphore_net::PeerEvent;
    let event = PeerEvent::PeerDisconnected {
        peer_id: PeerId::new("cafebabe"),
    };
    let dbg = format!("{event:?}");
    assert!(dbg.contains("PeerDisconnected"), "expected 'PeerDisconnected' in: {dbg}");
    assert!(dbg.contains("cafebabe"), "expected peer_id in: {dbg}");
}

// --- ConnectionControl tests ---

#[test]
fn connection_control_promote_trusted_is_debug() {
    use periphore_net::ConnectionControl;
    let ctrl = ConnectionControl::PromoteTrusted;
    let dbg = format!("{ctrl:?}");
    assert!(dbg.contains("PromoteTrusted"), "expected 'PromoteTrusted' in: {dbg}");
}

#[test]
fn connection_control_reject_is_debug() {
    use periphore_net::ConnectionControl;
    let ctrl = ConnectionControl::Reject;
    let dbg = format!("{ctrl:?}");
    assert!(dbg.contains("Reject"), "expected 'Reject' in: {dbg}");
}

// --- PendingPeer tests ---

#[test]
fn pending_peer_has_promote_tx_channel() {
    use periphore_net::{ConnectionControl, PendingPeer};
    use tokio::sync::mpsc;

    let (tx, _rx) = mpsc::channel::<ConnectionControl>(1);
    let pending = PendingPeer {
        fingerprint_hex: "abc".to_string(),
        identicon: "icon".to_string(),
        word_phrase: vec!["word1".to_string()],
        promote_tx: tx,
    };
    let dbg = format!("{pending:?}");
    assert!(dbg.contains("PendingPeer"), "expected 'PendingPeer' in: {dbg}");
    assert!(dbg.contains("abc"), "expected fingerprint_hex in: {dbg}");
}

// --- ActiveConn tests ---

#[test]
fn active_conn_has_peer_id() {
    use periphore_core::PeerId;
    use periphore_net::ActiveConn;

    let conn = ActiveConn {
        peer_id: PeerId::new("1234abcd"),
    };
    let dbg = format!("{conn:?}");
    assert!(dbg.contains("ActiveConn"), "expected 'ActiveConn' in: {dbg}");
    assert!(dbg.contains("1234abcd"), "expected peer_id in: {dbg}");
}

// --- HandshakeResult tests ---

#[test]
fn handshake_result_trusted_variant() {
    use periphore_core::PeerId;
    use periphore_net::HandshakeResult;

    let result = HandshakeResult::Trusted {
        peer_id: PeerId::new("trusted_fp"),
        fingerprint_hex: "trusted_fp".to_string(),
    };
    let dbg = format!("{result:?}");
    assert!(dbg.contains("Trusted"), "expected 'Trusted' in: {dbg}");
}

#[test]
fn handshake_result_pending_variant() {
    use periphore_core::PeerId;
    use periphore_net::HandshakeResult;

    let result = HandshakeResult::Pending {
        peer_id: PeerId::new("pending_fp"),
        fingerprint_hex: "pending_fp".to_string(),
        identicon: "icon".to_string(),
        word_phrase: vec!["one".to_string()],
    };
    let dbg = format!("{result:?}");
    assert!(dbg.contains("Pending"), "expected 'Pending' in: {dbg}");
    assert!(dbg.contains("pending_fp"), "expected fingerprint in: {dbg}");
}

#[test]
fn handshake_result_rejected_variant() {
    use periphore_net::HandshakeResult;

    let result = HandshakeResult::Rejected {
        reason: "version mismatch".to_string(),
    };
    let dbg = format!("{result:?}");
    assert!(dbg.contains("Rejected"), "expected 'Rejected' in: {dbg}");
    assert!(dbg.contains("version mismatch"), "expected reason in: {dbg}");
}

// --- DEFAULT_PORT test ---

#[test]
fn default_port_is_7888() {
    use periphore_net::DEFAULT_PORT;
    assert_eq!(DEFAULT_PORT, 7888, "DEFAULT_PORT must be 7888 per D-08");
}
