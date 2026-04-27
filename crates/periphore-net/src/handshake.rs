//! periphore-net handshake: Hello/HelloAck protocol state machine.
//!
//! T-6-02: Unknown peers return HandshakeResult::Pending — no input forwarding
//!         until ConnectionControl::PromoteTrusted received.
//! T-6-03: Protocol version mismatch → HelloAck { accepted: false } + NetError::ProtocolVersion.
//! T-6-04: Fingerprint conflict → HelloAck { accepted: false } + NetError::FingerprintConflict.
//! T-6-02: 10-second timeout on every receive — hung/malicious peer must not block task forever.

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt as _, StreamExt as _};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

use periphore_config::PeerConfig;
use periphore_core::PeerId;
use periphore_identity::IdentityStore;
use periphore_protocol::PeerMessage;
use periphore_trust::TrustStore;

use crate::codec::{decode_message, encode_message};
use crate::connection::HandshakeResult;
use crate::error::NetError;

/// Wire protocol version. Both sides must match or the connection is refused.
/// Version mismatch: send HelloAck { accepted: false } and disconnect (T-6-03).
pub const PROTOCOL_VERSION: u32 = 1;

/// Timeout for each read step in the handshake (T-6-02 mitigation).
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Convert a 32-byte array to lowercase hex string (no external dependency).
fn bytes_to_hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Perform the handshake as the initiating (outbound connector) side.
///
/// Protocol:
/// 1. Send Hello with our fingerprint + public_key.
/// 2. Receive HelloAck from responder.
/// 3. Check version acceptance, fingerprint conflict, and trust store.
/// 4. Return HandshakeResult::Trusted, ::Pending, or ::Rejected.
///
/// T-6-03: If the responder sends HelloAck { accepted: false }, returns Rejected.
/// T-6-04: If peer_config.fingerprint is set and conflicts, returns Err(FingerprintConflict).
/// D-02: Pending result carries real identicon and word_phrase from the peer's fingerprint.
pub async fn perform_handshake_initiator(
    framed_read: &mut FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
    framed_write: &mut FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
    identity: &IdentityStore,
    trust_store: &Arc<std::sync::RwLock<TrustStore>>,
    peer_config: Option<&PeerConfig>,
) -> Result<HandshakeResult, NetError> {
    // Step 1: Send Hello
    let hello = PeerMessage::Hello {
        protocol_version: PROTOCOL_VERSION,
        fingerprint: identity.fingerprint,
        public_key: identity.keypair.verifying_key().to_bytes().to_vec(),
    };
    framed_write
        .send(encode_message(&hello)?)
        .await
        .map_err(NetError::Io)?;

    // Step 2: Receive HelloAck with timeout (T-6-02 mitigation)
    let frame = tokio::time::timeout(HANDSHAKE_TIMEOUT, framed_read.next())
        .await
        .map_err(|_| NetError::ConnectionClosed)?
        .ok_or(NetError::ConnectionClosed)?
        .map_err(NetError::Io)?;

    let msg = decode_message(frame)?;

    let (peer_fp, _peer_pk, accepted) = match msg {
        PeerMessage::HelloAck {
            fingerprint,
            public_key,
            accepted,
        } => (fingerprint, public_key, accepted),
        other => {
            return Err(NetError::UnexpectedMessage(format!(
                "expected HelloAck, got {:?}",
                std::mem::discriminant(&other)
            )));
        }
    };

    // Step 3: Check if peer accepted us
    if !accepted {
        tracing::warn!("peer rejected our Hello (protocol version mismatch or fingerprint conflict on their side)");
        return Ok(HandshakeResult::Rejected {
            reason: "peer rejected our hello".into(),
        });
    }

    let peer_fp_hex = bytes_to_hex(&peer_fp);
    let peer_id = PeerId::new(peer_fp_hex.clone());

    // Step 4: Check configured fingerprint conflict (T-6-04)
    if let Some(cfg) = peer_config {
        if let Some(configured_fp) = &cfg.fingerprint {
            let label = cfg
                .name
                .as_deref()
                .unwrap_or(&peer_fp_hex[..8]);
            periphore_trust::check_peer_fingerprint(configured_fp, &peer_fp_hex, label)
                .map_err(|e| NetError::FingerprintConflict(e.to_string()))?;
        }
    }

    // Step 5: Check trust store
    let is_trusted = trust_store
        .read()
        .map_err(|_| NetError::Decode("trust lock poisoned".into()))?
        .is_trusted(&peer_fp_hex);

    if is_trusted {
        Ok(HandshakeResult::Trusted {
            peer_id,
            fingerprint_hex: peer_fp_hex,
        })
    } else {
        // D-02: compute identicon and word_phrase for the PEER's fingerprint (not ours).
        // Uses periphore_identity free functions added in this plan's pre-step.
        let identicon = periphore_identity::identicon_from_fingerprint(&peer_fp);
        let word_phrase = periphore_identity::word_phrase_from_fingerprint(&peer_fp);
        Ok(HandshakeResult::Pending {
            peer_id,
            fingerprint_hex: peer_fp_hex,
            identicon,
            word_phrase,
        })
    }
}

/// Perform the handshake as the responding (inbound accept) side.
///
/// Protocol:
/// 1. Receive Hello from initiator.
/// 2. Check protocol version — mismatch → send HelloAck { accepted: false } + return Err.
/// 3. Check fingerprint conflict (if peer_config has configured fingerprint).
/// 4. Send HelloAck { accepted: true }.
/// 5. Check trust store → return HandshakeResult.
///
/// T-6-03: Protocol version mismatch → send rejection and return Err(ProtocolVersion).
/// T-6-04: Fingerprint conflict → send rejection and return Err(FingerprintConflict).
/// D-02: Pending result carries real identicon and word_phrase from the peer's fingerprint.
pub async fn perform_handshake_responder(
    framed_read: &mut FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
    framed_write: &mut FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
    identity: &IdentityStore,
    trust_store: &Arc<std::sync::RwLock<TrustStore>>,
    peer_config: Option<&PeerConfig>,
) -> Result<HandshakeResult, NetError> {
    // Step 1: Receive Hello with timeout (T-6-02 mitigation)
    let frame = tokio::time::timeout(HANDSHAKE_TIMEOUT, framed_read.next())
        .await
        .map_err(|_| NetError::ConnectionClosed)?
        .ok_or(NetError::ConnectionClosed)?
        .map_err(NetError::Io)?;

    let msg = decode_message(frame)?;

    let (protocol_version, peer_fp, _peer_pk) = match msg {
        PeerMessage::Hello {
            protocol_version,
            fingerprint,
            public_key,
        } => (protocol_version, fingerprint, public_key),
        other => {
            return Err(NetError::UnexpectedMessage(format!(
                "expected Hello, got {:?}",
                std::mem::discriminant(&other)
            )));
        }
    };

    let peer_fp_hex = bytes_to_hex(&peer_fp);
    let peer_id = PeerId::new(peer_fp_hex.clone());

    // Step 2: Check protocol version (T-6-03)
    if protocol_version != PROTOCOL_VERSION {
        let rejection = PeerMessage::HelloAck {
            fingerprint: identity.fingerprint,
            public_key: identity.keypair.verifying_key().to_bytes().to_vec(),
            accepted: false,
        };
        framed_write
            .send(encode_message(&rejection)?)
            .await
            .map_err(NetError::Io)?;
        framed_write.flush().await.map_err(NetError::Io)?;
        return Err(NetError::ProtocolVersion {
            expected: PROTOCOL_VERSION,
            got: protocol_version,
        });
    }

    // Step 3: Check configured fingerprint conflict (T-6-04)
    if let Some(cfg) = peer_config {
        if let Some(configured_fp) = &cfg.fingerprint {
            let label = cfg
                .name
                .as_deref()
                .unwrap_or(&peer_fp_hex[..8]);
            if let Err(e) =
                periphore_trust::check_peer_fingerprint(configured_fp, &peer_fp_hex, label)
            {
                let rejection = PeerMessage::HelloAck {
                    fingerprint: identity.fingerprint,
                    public_key: identity.keypair.verifying_key().to_bytes().to_vec(),
                    accepted: false,
                };
                framed_write
                    .send(encode_message(&rejection)?)
                    .await
                    .map_err(NetError::Io)?;
                framed_write.flush().await.map_err(NetError::Io)?;
                return Err(NetError::FingerprintConflict(e.to_string()));
            }
        }
    }

    // Step 4: Send HelloAck { accepted: true }
    let ack = PeerMessage::HelloAck {
        fingerprint: identity.fingerprint,
        public_key: identity.keypair.verifying_key().to_bytes().to_vec(),
        accepted: true,
    };
    framed_write
        .send(encode_message(&ack)?)
        .await
        .map_err(NetError::Io)?;
    framed_write.flush().await.map_err(NetError::Io)?;

    // Step 5: Check trust store
    let is_trusted = trust_store
        .read()
        .map_err(|_| NetError::Decode("trust lock poisoned".into()))?
        .is_trusted(&peer_fp_hex);

    if is_trusted {
        Ok(HandshakeResult::Trusted {
            peer_id,
            fingerprint_hex: peer_fp_hex,
        })
    } else {
        // D-02: compute identicon and word_phrase for the PEER's fingerprint (not ours).
        // Uses periphore_identity free functions added in this plan's pre-step.
        let identicon = periphore_identity::identicon_from_fingerprint(&peer_fp);
        let word_phrase = periphore_identity::word_phrase_from_fingerprint(&peer_fp);
        Ok(HandshakeResult::Pending {
            peer_id,
            fingerprint_hex: peer_fp_hex,
            identicon,
            word_phrase,
        })
    }
}
