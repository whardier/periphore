//! periphore-net connection manager: accept loop + outbound connector + pending/active state.
//!
//! D-19: TCP_NODELAY is set IMMEDIATELY after accept()/connect() — before any other socket
//!       operation, before split_framed(). This is a hard CLAUDE.md requirement.
//! D-09: Exponential backoff connector: 1s→2s→4s→8s→16s→cap 30s.
//! T-6-05: Every backoff sleep is inside tokio::select! { _ = token.cancelled() => return }.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use futures_util::StreamExt as _;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use periphore_config::PeerConfig;
use periphore_identity::IdentityStore;
use periphore_protocol::PendingPeerInfo;
use periphore_trust::TrustStore;

use crate::{
    codec,
    connection::{ActiveConn, ConnectionControl, HandshakeResult, PendingPeer},
    error::NetError,
    event::PeerEvent,
    handshake,
    DEFAULT_PORT,
};

/// Initial backoff delay for outbound connector retry (D-09, T-6-05).
const BACKOFF_INITIAL_MS: u64 = 1_000;
/// Maximum backoff delay cap — doubles each attempt until this limit (D-09, T-6-05).
const BACKOFF_CAP_MS: u64 = 30_000;

/// Manages inbound and outbound TCP peer connections.
///
/// Owns the TCP sockets, executes handshakes, and emits `PeerEvent` notifications
/// to the daemon's `select!` loop. The daemon imports and uses `ConnectionManager`
/// after Phase 6 wires `periphore-net` into `periphored`.
///
/// Pending peer state is shared between spawned tasks and the manager via
/// `Arc<Mutex<HashMap>>` so both sides can access `PendingPeer` concurrently.
pub struct ConnectionManager {
    event_tx: mpsc::Sender<PeerEvent>,
    /// Per-peer cancellation tokens — keyed by peer name or host string.
    peer_tokens: HashMap<String, CancellationToken>,
    /// Pending peers awaiting user acceptance — keyed by fingerprint_hex.
    /// Shared with spawned connector/acceptor tasks via Arc.
    pending: Arc<std::sync::Mutex<HashMap<String, PendingPeer>>>,
    /// Active (trusted) connections — keyed by fingerprint_hex.
    active: HashMap<String, ActiveConn>,
}

impl ConnectionManager {
    /// Create a new `ConnectionManager` with empty connection maps.
    ///
    /// `event_tx` is the channel the manager uses to notify the daemon of peer events.
    pub fn new(event_tx: mpsc::Sender<PeerEvent>) -> Self {
        Self {
            event_tx,
            peer_tokens: HashMap::new(),
            pending: Arc::new(std::sync::Mutex::new(HashMap::new())),
            active: HashMap::new(),
        }
    }

    /// Bind a TCP listener and spawn the accept loop task into `tasks`.
    ///
    /// The accept loop runs indefinitely:
    /// - Accept a connection.
    /// - IMMEDIATELY set TCP_NODELAY (D-19 hard requirement).
    /// - Spawn a task to run perform_handshake_responder.
    /// - On handshake result, emit PeerEvent.
    /// - A single accept error never aborts the loop (mirrors IPC server.rs pattern).
    pub fn spawn_listener(
        &mut self,
        tasks: &mut JoinSet<anyhow::Result<()>>,
        bind_addr: SocketAddr,
        identity: Arc<IdentityStore>,
        trust_store: Arc<RwLock<TrustStore>>,
    ) {
        let event_tx = self.event_tx.clone();
        let pending = Arc::clone(&self.pending);

        tasks.spawn(async move {
            let listener = TcpListener::bind(bind_addr)
                .await
                .map_err(|e| anyhow::anyhow!("TCP listener bind error on {bind_addr}: {e}"))?;
            tracing::info!(addr = %bind_addr, "TCP peer listener bound");

            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        // D-19 HARD REQUIREMENT: TCP_NODELAY immediately after accept,
                        // before any other socket operation.
                        if let Err(e) = stream.set_nodelay(true) {
                            tracing::error!(
                                addr = %peer_addr,
                                error = %e,
                                "TCP_NODELAY failed on accepted connection — dropping"
                            );
                            continue;
                        }

                        tracing::info!(addr = %peer_addr, "accepted TCP connection");

                        let (mut framed_read, mut framed_write) = codec::split_framed(stream);
                        let identity = Arc::clone(&identity);
                        let trust_store = Arc::clone(&trust_store);
                        let event_tx = event_tx.clone();
                        let pending = Arc::clone(&pending);

                        tokio::spawn(async move {
                            match handshake::perform_handshake_responder(
                                &mut framed_read,
                                &mut framed_write,
                                &identity,
                                &trust_store,
                                None,
                            )
                            .await
                            {
                                Ok(HandshakeResult::Trusted { peer_id, .. }) => {
                                    tracing::info!(
                                        addr = %peer_addr,
                                        peer_id = %peer_id,
                                        "inbound peer trusted"
                                    );
                                    event_tx
                                        .send(PeerEvent::PeerConnected {
                                            peer_id: peer_id.clone(),
                                        })
                                        .await
                                        .ok();
                                    // Phase 6: hold connection open until EOF/error
                                    loop {
                                        match tokio::time::timeout(
                                            Duration::from_secs(30),
                                            framed_read.next(),
                                        )
                                        .await
                                        {
                                            Ok(Some(Ok(_frame))) => {
                                                // Ignore non-handshake frames in Phase 6
                                            }
                                            _ => break,
                                        }
                                    }
                                    event_tx
                                        .send(PeerEvent::PeerDisconnected { peer_id })
                                        .await
                                        .ok();
                                }
                                Ok(HandshakeResult::Pending {
                                    peer_id,
                                    fingerprint_hex,
                                    identicon,
                                    word_phrase,
                                }) => {
                                    tracing::warn!(
                                        addr = %peer_addr,
                                        fingerprint = %fingerprint_hex,
                                        "inbound peer is pending (unknown fingerprint)"
                                    );
                                    let (promote_tx, mut promote_rx) =
                                        mpsc::channel::<ConnectionControl>(1);
                                    {
                                        let mut guard = pending.lock().unwrap_or_else(|e| e.into_inner());
                                        guard.insert(
                                            fingerprint_hex.clone(),
                                            PendingPeer {
                                                fingerprint_hex: fingerprint_hex.clone(),
                                                identicon: identicon.clone(),
                                                word_phrase: word_phrase.clone(),
                                                promote_tx,
                                            },
                                        );
                                    }
                                    event_tx
                                        .send(PeerEvent::PeerPending {
                                            fingerprint: fingerprint_hex.clone(),
                                            identicon,
                                            word_phrase,
                                        })
                                        .await
                                        .ok();
                                    match promote_rx.recv().await {
                                        Some(ConnectionControl::PromoteTrusted) => {
                                            event_tx
                                                .send(PeerEvent::PeerConnected {
                                                    peer_id: peer_id.clone(),
                                                })
                                                .await
                                                .ok();
                                            // Hold connection open until EOF/error (same as Trusted path)
                                            loop {
                                                match tokio::time::timeout(
                                                    Duration::from_secs(30),
                                                    framed_read.next(),
                                                )
                                                .await
                                                {
                                                    Ok(Some(Ok(_frame))) => {}
                                                    _ => break,
                                                }
                                            }
                                            event_tx
                                                .send(PeerEvent::PeerDisconnected { peer_id })
                                                .await
                                                .ok();
                                        }
                                        _ => {} // rejected or channel dropped
                                    }
                                    // Remove from pending map
                                    let mut guard =
                                        pending.lock().unwrap_or_else(|e| e.into_inner());
                                    guard.remove(&fingerprint_hex);
                                }
                                Ok(HandshakeResult::Rejected { reason }) => {
                                    tracing::warn!(
                                        addr = %peer_addr,
                                        reason = %reason,
                                        "inbound peer handshake rejected"
                                    );
                                }
                                Err(e) => {
                                    tracing::info!(
                                        addr = %peer_addr,
                                        error = %e,
                                        "inbound peer handshake failed"
                                    );
                                }
                            }
                        });
                    }
                    Err(e) => {
                        // Mirror server.rs pattern: log and continue — never abort the accept loop
                        tracing::error!(error = %e, "TCP accept error");
                    }
                }
            }
        });
    }

    /// Spawn an outbound connector task for `peer_config` into `tasks`.
    ///
    /// The task connects to the peer's host:port and executes the handshake. On failure or
    /// disconnection, retries with exponential backoff (1s→30s cap, D-09). Each retry checks
    /// the CancellationToken so the task exits promptly when `cancel_peer()` is called (T-6-05).
    ///
    /// The key used for `peer_tokens` is `peer_config.name` if set, otherwise
    /// `"host:port"` (matching the format used by the SIGHUP diff logic in periphored).
    /// Callers that need to cancel a connector should use the same key format.
    pub fn spawn_connector(
        &mut self,
        tasks: &mut JoinSet<anyhow::Result<()>>,
        peer_config: PeerConfig,
        identity: Arc<IdentityStore>,
        trust_store: Arc<RwLock<TrustStore>>,
    ) {
        let peer_key = peer_config.name.clone().unwrap_or_else(|| {
            let host = peer_config.host.as_deref().unwrap_or("");
            let port = peer_config.port.unwrap_or(DEFAULT_PORT);
            format!("{host}:{port}")
        });

        let token = CancellationToken::new();
        self.peer_tokens.insert(peer_key.clone(), token.clone());

        let event_tx = self.event_tx.clone();
        let pending = Arc::clone(&self.pending);

        tasks.spawn(async move {
            let host = peer_config
                .host
                .as_deref()
                .unwrap_or("127.0.0.1")
                .to_owned();
            let port = peer_config.port.unwrap_or(DEFAULT_PORT);
            let addr = format!("{host}:{port}");

            let mut delay_ms = BACKOFF_INITIAL_MS;

            loop {
                match TcpStream::connect(&addr).await {
                    Ok(stream) => {
                        // D-19 HARD REQUIREMENT: TCP_NODELAY immediately after connect,
                        // before any other socket operation.
                        if let Err(e) = stream.set_nodelay(true) {
                            tracing::error!(
                                addr = %addr,
                                error = %e,
                                "TCP_NODELAY failed — dropping connection, will retry"
                            );
                            // fall through to retry
                        } else {
                            delay_ms = BACKOFF_INITIAL_MS; // reset on successful connect
                            let (mut framed_read, mut framed_write) = codec::split_framed(stream);

                            match handshake::perform_handshake_initiator(
                                &mut framed_read,
                                &mut framed_write,
                                &identity,
                                &trust_store,
                                Some(&peer_config),
                            )
                            .await
                            {
                                Ok(HandshakeResult::Trusted { peer_id, .. }) => {
                                    tracing::info!(
                                        addr = %addr,
                                        peer_id = %peer_id,
                                        "outbound peer trusted"
                                    );
                                    event_tx
                                        .send(PeerEvent::PeerConnected {
                                            peer_id: peer_id.clone(),
                                        })
                                        .await
                                        .ok();
                                    // Phase 6: hold connection open until EOF/error
                                    loop {
                                        match tokio::time::timeout(
                                            Duration::from_secs(30),
                                            framed_read.next(),
                                        )
                                        .await
                                        {
                                            Ok(Some(Ok(_frame))) => {
                                                // Ignore non-handshake frames in Phase 6
                                            }
                                            _ => break,
                                        }
                                    }
                                    event_tx
                                        .send(PeerEvent::PeerDisconnected { peer_id })
                                        .await
                                        .ok();
                                }
                                Ok(HandshakeResult::Pending {
                                    peer_id,
                                    fingerprint_hex,
                                    identicon,
                                    word_phrase,
                                }) => {
                                    tracing::warn!(
                                        addr = %addr,
                                        fingerprint = %fingerprint_hex,
                                        "outbound peer is pending (unknown fingerprint)"
                                    );
                                    let (promote_tx, mut promote_rx) =
                                        mpsc::channel::<ConnectionControl>(1);
                                    {
                                        let mut guard =
                                            pending.lock().unwrap_or_else(|e| e.into_inner());
                                        guard.insert(
                                            fingerprint_hex.clone(),
                                            PendingPeer {
                                                fingerprint_hex: fingerprint_hex.clone(),
                                                identicon: identicon.clone(),
                                                word_phrase: word_phrase.clone(),
                                                promote_tx,
                                            },
                                        );
                                    }
                                    event_tx
                                        .send(PeerEvent::PeerPending {
                                            fingerprint: fingerprint_hex.clone(),
                                            identicon,
                                            word_phrase,
                                        })
                                        .await
                                        .ok();
                                    match promote_rx.recv().await {
                                        Some(ConnectionControl::PromoteTrusted) => {
                                            event_tx
                                                .send(PeerEvent::PeerConnected {
                                                    peer_id: peer_id.clone(),
                                                })
                                                .await
                                                .ok();
                                            // Hold connection open until EOF/error (same as Trusted path)
                                            loop {
                                                match tokio::time::timeout(
                                                    Duration::from_secs(30),
                                                    framed_read.next(),
                                                )
                                                .await
                                                {
                                                    Ok(Some(Ok(_frame))) => {}
                                                    _ => break,
                                                }
                                            }
                                            event_tx
                                                .send(PeerEvent::PeerDisconnected { peer_id })
                                                .await
                                                .ok();
                                        }
                                        _ => {} // rejected or channel dropped
                                    }
                                    // Remove from pending map
                                    let mut guard =
                                        pending.lock().unwrap_or_else(|e| e.into_inner());
                                    guard.remove(&fingerprint_hex);
                                }
                                Ok(HandshakeResult::Rejected { reason }) => {
                                    tracing::warn!(
                                        addr = %addr,
                                        reason = %reason,
                                        "outbound handshake rejected — will retry"
                                    );
                                }
                                Err(e) => {
                                    tracing::info!(
                                        addr = %addr,
                                        error = %e,
                                        "outbound handshake failed — will retry"
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::info!(
                            addr = %addr,
                            error = %e,
                            delay_ms,
                            "peer connection failed — retrying"
                        );
                    }
                }

                // T-6-05: Check cancellation before sleeping — removed peers exit promptly.
                tokio::select! {
                    _ = token.cancelled() => {
                        tracing::info!(peer = %peer_key, "connector task cancelled");
                        return Ok(());
                    }
                    _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {}
                }

                // Exponential backoff: double delay, cap at BACKOFF_CAP_MS (D-09)
                delay_ms = (delay_ms * 2).min(BACKOFF_CAP_MS);
            }
        });
    }

    /// Promote a pending peer to trusted by sending `ConnectionControl::PromoteTrusted`
    /// on the pending peer's channel.
    ///
    /// Returns `Ok(())` if the peer was found and the signal sent. If the peer is not in
    /// pending state (e.g., already reconnected as trusted), logs at INFO and returns `Ok(())`.
    pub async fn promote_pending(&self, fingerprint_hex: &str) -> Result<(), NetError> {
        let tx = {
            let guard = self
                .pending
                .lock()
                .map_err(|_| NetError::PeerNotFound(fingerprint_hex.to_owned()))?;
            guard.get(fingerprint_hex).map(|p| p.promote_tx.clone())
        };
        match tx {
            Some(ch) => ch
                .send(ConnectionControl::PromoteTrusted)
                .await
                .map_err(|_| NetError::PeerNotFound(fingerprint_hex.to_owned())),
            None => {
                // Peer not in pending — may have already connected or reconnected as trusted.
                // Log at INFO (not error) per RESEARCH.md Pitfall 4.
                tracing::info!(
                    fingerprint = %fingerprint_hex,
                    "promote_pending: peer not in pending state (may have already reconnected as trusted)"
                );
                Ok(())
            }
        }
    }

    /// Return the list of all peers currently in pending verification state.
    ///
    /// Used by `GetPendingVerifications` IPC dispatch in `periphored` to surface
    /// pending peers to the CLI (D-03).
    pub fn pending_list(&self) -> Vec<PendingPeerInfo> {
        let guard = self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard
            .values()
            .map(|p| PendingPeerInfo {
                fingerprint: p.fingerprint_hex.clone(),
                identicon: p.identicon.clone(),
                word_phrase: p.word_phrase.clone(),
            })
            .collect()
    }

    /// Cancel the connector task for the named peer key.
    ///
    /// The peer key is `peer_config.name` if set, otherwise `peer_config.host`. After
    /// cancellation the connector task exits at the next backoff sleep boundary (T-6-05).
    /// Used when a peer is removed from config via D-11 (config diff on SIGHUP).
    pub fn cancel_peer(&mut self, peer_key: &str) {
        if let Some(token) = self.peer_tokens.remove(peer_key) {
            token.cancel();
        }
    }
}
