//! SSH tunnel port probe loop.
//!
//! Periodically probes a range of localhost ports for SSH-forwarded Periphore daemons.
//! Uses the real Hello/HelloAck handshake to validate the remote is a compatible daemon.
//!
//! Pitfall 3: Skips own daemon via fingerprint comparison (HelloAck fingerprint == local fingerprint).
//! Pitfall 4: Disconnects immediately after HelloAck -- remote ConnectionManager cleans up.

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt as _, StreamExt as _};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use periphore_identity::IdentityStore;
use periphore_protocol::PeerMessage;

use crate::list::{DiscoveredPeerList, DiscoverySource};
use crate::DiscoveryEvent;

/// Maximum time to wait for a connection to localhost (fast timeout for unoccupied ports).
const PROBE_CONNECT_TIMEOUT: Duration = Duration::from_millis(100);
/// Maximum time to wait for HelloAck after sending Hello (generous for localhost).
const PROBE_HANDSHAKE_TIMEOUT: Duration = Duration::from_millis(200);
/// Interval between full port sweep cycles.
const PROBE_INTERVAL: Duration = Duration::from_secs(30);

/// Probe loop: periodically sweeps configured ports for SSH-forwarded Periphore daemons.
///
/// For each port:
/// 1. Attempt TCP connection with `PROBE_CONNECT_TIMEOUT` (100ms).
/// 2. If connected, set TCP_NODELAY immediately (CLAUDE.md hard requirement).
/// 3. Attempt Hello/HelloAck handshake.
/// 4. If HelloAck fingerprint matches own fingerprint: skip (self-detection, Pitfall 3).
/// 5. Otherwise: upsert into discovered list and emit PeerDiscovered.
///
/// Between sweeps, waits `PROBE_INTERVAL` (30s) with cancellation support.
pub(crate) async fn ssh_probe_loop(
    ports: Vec<u16>,
    own_fingerprint: [u8; 32],
    identity: Arc<IdentityStore>,
    peers: Arc<std::sync::Mutex<DiscoveredPeerList>>,
    event_tx: mpsc::Sender<DiscoveryEvent>,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    tracing::info!(
        port_count = ports.len(),
        interval_secs = PROBE_INTERVAL.as_secs(),
        "SSH tunnel port probe loop started"
    );

    loop {
        for &port in &ports {
            // Fast connect timeout for localhost
            match tokio::time::timeout(
                PROBE_CONNECT_TIMEOUT,
                TcpStream::connect(("127.0.0.1", port)),
            )
            .await
            {
                Ok(Ok(stream)) => {
                    // D-19 HARD REQUIREMENT: TCP_NODELAY immediately after connect.
                    if let Err(e) = stream.set_nodelay(true) {
                        tracing::trace!(port, error = %e, "probe: TCP_NODELAY failed — skipping port");
                        continue;
                    }

                    match probe_handshake(stream, &identity).await {
                        Ok(Some(peer_fingerprint)) => {
                            // Pitfall 3: skip if this is our own daemon
                            if peer_fingerprint == own_fingerprint {
                                tracing::trace!(
                                    port,
                                    "probe: skipping self-discovered daemon (fingerprint match)"
                                );
                                continue;
                            }

                            tracing::debug!(port, "probe: SSH-forwarded Periphore daemon discovered");

                            peers
                                .lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .upsert(
                                    "127.0.0.1".to_owned(),
                                    port,
                                    DiscoverySource::SshProbe,
                                    None,
                                );

                            event_tx
                                .send(DiscoveryEvent::PeerDiscovered {
                                    hostname: "127.0.0.1".to_owned(),
                                    port,
                                    source: DiscoverySource::SshProbe,
                                })
                                .await
                                .ok();
                        }
                        Ok(None) => {
                            tracing::trace!(
                                port,
                                "probe: port connected but not a compatible Periphore daemon"
                            );
                        }
                        Err(e) => {
                            tracing::trace!(port, error = %e, "probe: handshake failed");
                        }
                    }
                }
                // Timeout or connection refused — normal for unoccupied ports
                Ok(Err(_)) | Err(_) => {}
            }
        }

        // Wait PROBE_INTERVAL before next sweep, with cancellation support.
        tokio::select! {
            _ = cancel.cancelled() => {
                tracing::debug!("SSH probe loop cancelled");
                break;
            }
            _ = tokio::time::sleep(PROBE_INTERVAL) => {}
        }
    }

    Ok(())
}

/// Perform a lightweight Hello/HelloAck handshake to identify a Periphore daemon.
///
/// Returns:
/// - `Ok(Some([u8; 32]))` if the remote responded with HelloAck { accepted: true }
///   (the fingerprint is the remote's fingerprint for self-detection comparison).
/// - `Ok(None)` if the port connected but is not a Periphore daemon
///   (wrong protocol, version mismatch, or rejected).
/// - `Err(...)` on I/O or framing error.
///
/// Connection drops immediately when this function returns (Pitfall 4 mitigation).
async fn probe_handshake(
    stream: TcpStream,
    identity: &IdentityStore,
) -> anyhow::Result<Option<[u8; 32]>> {
    let (mut fr, mut fw) = periphore_net::codec::split_framed(stream);

    // Send Hello with our identity
    // NOTE: identity.keypair is pub (WR-01 open TODO) — accessed directly here.
    // When WR-01 is resolved with sign()/verifying_key() accessors, update this.
    let hello = PeerMessage::Hello {
        protocol_version: periphore_net::PROTOCOL_VERSION,
        fingerprint: identity.fingerprint,
        public_key: identity.keypair.verifying_key().to_bytes().to_vec(),
    };
    fw.send(periphore_net::codec::encode_message(&hello)?).await?;

    // Read HelloAck with tight timeout (200ms is generous for localhost)
    let frame = tokio::time::timeout(PROBE_HANDSHAKE_TIMEOUT, fr.next())
        .await
        .map_err(|_| anyhow::anyhow!("probe handshake timeout"))?
        .ok_or_else(|| anyhow::anyhow!("connection closed before HelloAck"))?
        .map_err(|e| anyhow::anyhow!("frame read error: {e}"))?;

    let msg = periphore_net::codec::decode_message(frame)?;

    match msg {
        PeerMessage::HelloAck {
            fingerprint,
            accepted,
            ..
        } => {
            if accepted {
                Ok(Some(fingerprint))
            } else {
                // Remote rejected us (version mismatch on their side, or other)
                Ok(None)
            }
        }
        _ => {
            // Unexpected message type — not a compatible Periphore daemon
            Ok(None)
        }
    }
}
