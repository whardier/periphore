//! Round-trip serialization tests for periphore-protocol.
//! PeerMessage: postcard serialization (wire protocol)
//! IpcRequest/IpcResponse: serde_json serialization (IPC protocol)

use periphore_protocol::{
    DiscoveredPeerInfo, Edge, EdgeMapping, InputEvent, IpcRequest, IpcResponse, KeyEventData,
    MonitorInfo, MouseEventData, PeerMessage, PendingPeerInfo,
};

// -- PeerMessage postcard round-trip --

fn peer_round_trip(msg: PeerMessage) -> PeerMessage {
    let bytes: Vec<u8> = postcard::to_allocvec(&msg).expect("postcard serialize failed");
    postcard::from_bytes(&bytes).expect("postcard deserialize failed")
}

#[test]
fn peer_message_all_variants_round_trip() {
    let cases = vec![
        PeerMessage::Hello {
            protocol_version: 1,
            fingerprint:      [0u8; 32],
            public_key:       vec![1, 2, 3],
        },
        PeerMessage::HelloAck {
            fingerprint: [0xFFu8; 32],
            public_key:  vec![4, 5, 6],
            accepted:    true,
        },
        PeerMessage::TopologyAdvertise {
            monitors: vec![MonitorInfo {
                id:     0,
                width:  1920,
                height: 1080,
                x:      0,
                y:      0,
            }],
        },
        PeerMessage::TopologyPropose {
            edges: vec![EdgeMapping {
                from_monitor: 0,
                from_edge:    Edge::Right,
                to_peer:      "deadbeef".to_owned(),
                to_monitor:   0,
                to_edge:      Edge::Left,
            }],
        },
        PeerMessage::TopologyAccept,
        PeerMessage::TopologyReject {
            reason: "conflict".to_owned(),
        },
        PeerMessage::FocusTransfer {
            entry_edge:     Edge::Right,
            entry_position: 0.5,
            sequence:       42,
        },
        PeerMessage::FocusAck { sequence: 42 },
        PeerMessage::FocusReclaim,
        PeerMessage::MouseMove { dx: -100, dy: 200 },
        PeerMessage::MouseButton {
            button:  0,
            pressed: true,
        },
        PeerMessage::MouseScroll { dx: 0, dy: -3 },
        PeerMessage::KeyEvent {
            scancode:  0x1E,
            pressed:   true,
            modifiers: 0,
        },
        PeerMessage::Ping { timestamp: 12345 },
        PeerMessage::Pong { timestamp: 12345 },
        PeerMessage::Bye,
    ];
    for msg in cases {
        assert_eq!(
            msg,
            peer_round_trip(msg.clone()),
            "PeerMessage round-trip failed"
        );
    }
}

// -- IpcRequest serde_json round-trip --

fn ipc_req_round_trip(req: &IpcRequest) -> IpcRequest {
    let json = serde_json::to_string(req).expect("serde_json serialize failed");
    serde_json::from_str(&json).expect("serde_json deserialize failed")
}

fn ipc_resp_round_trip(resp: &IpcResponse) -> IpcResponse {
    let json = serde_json::to_string(resp).expect("serde_json serialize failed");
    serde_json::from_str(&json).expect("serde_json deserialize failed")
}

#[test]
fn ipc_request_all_variants_round_trip() {
    let cases: Vec<IpcRequest> = vec![
        IpcRequest::GetStatus,
        IpcRequest::ListPeers,
        IpcRequest::GetTopology,
        IpcRequest::AcceptFingerprint {
            fingerprint: "abc123".to_owned(),
        },
        IpcRequest::RejectFingerprint {
            fingerprint: "abc123".to_owned(),
        },
        IpcRequest::ReloadConfig,
        IpcRequest::InjectInputEvent {
            event: InputEvent::Mouse(MouseEventData { dx: 10, dy: -5 }),
        },
        IpcRequest::InjectInputEvent {
            event: InputEvent::Key(KeyEventData {
                scancode:  0x1E,
                pressed:   true,
                modifiers: 0,
            }),
        },
        IpcRequest::SimulateEdgeCross {
            edge:     Edge::Right,
            position: 0.5,
        },
        IpcRequest::GetState,
        IpcRequest::GetPendingVerifications,
        IpcRequest::GetDiscoveredPeers,
        IpcRequest::GetIdenticon {
            fingerprint: "abc123".to_owned(),
        },
        IpcRequest::GetWordPhrase {
            fingerprint: "abc123".to_owned(),
        },
    ];
    for req in &cases {
        let decoded = ipc_req_round_trip(req);
        let original_json =
            serde_json::to_string(req).expect("serialize original failed");
        let decoded_json =
            serde_json::to_string(&decoded).expect("serialize decoded failed");
        assert_eq!(
            original_json, decoded_json,
            "IpcRequest round-trip JSON mismatch"
        );
    }
}

#[test]
fn ipc_response_all_variants_round_trip() {
    let cases: Vec<IpcResponse> = vec![
        IpcResponse::Status {
            running:     true,
            fingerprint: None,
        },
        IpcResponse::Status {
            running:     false,
            fingerprint: Some("abc123".to_owned()),
        },
        IpcResponse::Peers {
            peers: vec!["fp1".to_owned(), "fp2".to_owned()],
        },
        IpcResponse::Identicon {
            fingerprint_hex: "a3f92b1e".to_owned(),
            identicon: "+--[ED25519 256]--+\n|      .S        |\n+--[PERIPHORE]----+\n".to_owned(),
        },
        IpcResponse::WordPhrase {
            words:  vec!["abandon".to_owned(), "ability".to_owned(), "able".to_owned(),
                         "about".to_owned(), "above".to_owned(), "absent".to_owned()],
            phrase: "abandon ability able about above absent".to_owned(),
        },
        IpcResponse::DiscoveredPeers {
            peers: vec![DiscoveredPeerInfo {
                hostname:        "peer.local".to_owned(),
                port:            7888,
                last_seen_epoch: 1_700_000_000,
                source:          "mdns".to_owned(),
            }],
        },
        IpcResponse::Ok,
        IpcResponse::Error {
            message: "something went wrong".to_owned(),
        },
    ];
    for resp in &cases {
        let decoded = ipc_resp_round_trip(resp);
        let original_json =
            serde_json::to_string(resp).expect("serialize original failed");
        let decoded_json =
            serde_json::to_string(&decoded).expect("serialize decoded failed");
        assert_eq!(
            original_json, decoded_json,
            "IpcResponse round-trip JSON mismatch"
        );
    }
}

/// Phase 6 D-03: PendingPeerInfo + IpcResponse::PendingPeers round-trip
#[test]
fn ipc_response_pending_peers_round_trip() {
    let info = PendingPeerInfo {
        fingerprint: "a3f92b1e".repeat(8),
        identicon: "+--[ED25519 256]--+\n|      .S        |\n+--[PERIPHORE]----+\n".to_owned(),
        word_phrase: vec![
            "abandon".to_owned(),
            "ability".to_owned(),
            "able".to_owned(),
            "about".to_owned(),
            "above".to_owned(),
            "absent".to_owned(),
        ],
    };
    let resp = IpcResponse::PendingPeers {
        peers: vec![info],
    };
    let json = serde_json::to_string(&resp).expect("serialize failed");
    assert!(
        json.contains("\"type\":\"pending_peers\""),
        "JSON must contain type:pending_peers tag: {json}"
    );
    assert!(
        json.contains("\"fingerprint\""),
        "JSON must contain fingerprint field: {json}"
    );
    let decoded: IpcResponse = serde_json::from_str(&json).expect("deserialize failed");
    let decoded_json = serde_json::to_string(&decoded).expect("re-serialize failed");
    assert_eq!(json, decoded_json, "PendingPeers round-trip JSON mismatch");
}

#[test]
fn ipc_inject_input_event_json_structure() {
    // Verify the JSON tag-based structure is correct for IPC client tooling
    let req = IpcRequest::InjectInputEvent {
        event: InputEvent::Mouse(MouseEventData { dx: 10, dy: -5 }),
    };
    let json = serde_json::to_string(&req).expect("serialize failed");
    // JSON should contain the "type" tag field (serde tag="type")
    assert!(
        json.contains("\"type\""),
        "JSON must contain 'type' field: {json}"
    );
    assert!(
        json.contains("inject_input_event"),
        "JSON must contain 'inject_input_event': {json}"
    );
}
