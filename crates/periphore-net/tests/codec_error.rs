/// TDD tests for periphore-net error.rs and codec.rs (Plan 06-02, Task 1).
///
/// RED phase: these tests are written before the implementation exists.
/// They define the exact contract that error.rs and codec.rs must satisfy.

// --- NetError tests ---

/// NetError must derive Debug and display meaningful messages.
#[test]
fn net_error_io_wraps_std_io_error() {
    use periphore_net::NetError;
    use std::io;
    let io_err = io::Error::new(io::ErrorKind::ConnectionRefused, "refused");
    let net_err: NetError = io_err.into();
    let msg = net_err.to_string();
    assert!(msg.contains("I/O error"), "expected 'I/O error' in: {msg}");
}

#[test]
fn net_error_protocol_version_has_expected_and_got_fields() {
    use periphore_net::NetError;
    let err = NetError::ProtocolVersion { expected: 1, got: 2 };
    let msg = err.to_string();
    assert!(msg.contains("1"), "expected '1' in: {msg}");
    assert!(msg.contains("2"), "expected '2' in: {msg}");
}

#[test]
fn net_error_peer_not_found_carries_label() {
    use periphore_net::NetError;
    let err = NetError::PeerNotFound("abc123".to_string());
    let msg = err.to_string();
    assert!(msg.contains("abc123"), "expected fingerprint in: {msg}");
}

#[test]
fn net_error_fingerprint_conflict_carries_description() {
    use periphore_net::NetError;
    let err = NetError::FingerprintConflict("expected A got B".to_string());
    let msg = err.to_string();
    assert!(msg.contains("fingerprint conflict"), "expected 'fingerprint conflict' in: {msg}");
}

#[test]
fn net_error_connection_closed_displays() {
    use periphore_net::NetError;
    let err = NetError::ConnectionClosed;
    let msg = err.to_string();
    assert!(msg.contains("connection closed"), "expected 'connection closed' in: {msg}");
}

// --- codec tests ---

/// encode_message / decode_message round-trip for a Hello message.
#[test]
fn codec_round_trips_peer_message_hello() {
    use periphore_net::codec::{decode_message, encode_message};
    use periphore_protocol::PeerMessage;
    use bytes::BytesMut;

    let msg = PeerMessage::Hello {
        protocol_version: 1,
        fingerprint: [0u8; 32],
        public_key: vec![1, 2, 3],
    };
    let encoded = encode_message(&msg).expect("encode_message must succeed");
    let frame = BytesMut::from(encoded.as_ref());
    let decoded = decode_message(frame).expect("decode_message must succeed");
    assert_eq!(msg, decoded, "round-trip must produce identical message");
}

/// encode_message / decode_message round-trip for the Bye variant (no fields).
#[test]
fn codec_round_trips_peer_message_bye() {
    use periphore_net::codec::{decode_message, encode_message};
    use periphore_protocol::PeerMessage;
    use bytes::BytesMut;

    let msg = PeerMessage::Bye;
    let encoded = encode_message(&msg).expect("encode_message must succeed for Bye");
    let frame = BytesMut::from(encoded.as_ref());
    let decoded = decode_message(frame).expect("decode_message must succeed for Bye");
    assert_eq!(msg, decoded, "Bye must round-trip correctly");
}

/// MAX_FRAME_LENGTH constant must equal 64*1024.
#[test]
fn codec_max_frame_length_is_64k() {
    use periphore_net::MAX_FRAME_LENGTH;
    assert_eq!(MAX_FRAME_LENGTH, 64 * 1024, "MAX_FRAME_LENGTH must be 64*1024 per T-6-01");
}

/// split_framed returns two independent halves (compilation test).
/// We test via in-memory TcpStream using tokio's mock infrastructure.
#[tokio::test]
async fn split_framed_returns_two_halves() {
    use periphore_net::codec::split_framed;

    // Create a real TCP pair for testing.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let client_task = tokio::spawn(async move {
        let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream.set_nodelay(true).unwrap();
        let (_read, _write) = split_framed(stream);
        // Both halves returned without panic.
    });

    let (server_stream, _) = listener.accept().await.unwrap();
    server_stream.set_nodelay(true).unwrap();
    let (_read, _write) = split_framed(server_stream);

    client_task.await.unwrap();
}
