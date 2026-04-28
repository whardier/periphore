//! periphore-discovery: mDNS and SSH tunnel peer discovery for Periphore.
//!
//! Provides:
//! - `DiscoveryService`: manages mDNS registration/browsing, SSH port probing, and TTL GC
//! - `DiscoveryEvent`: one-way notifications from discovery to the daemon
//!
//! D-01: Discovery logic lives in this dedicated crate (not periphore-net or periphored).
//! D-02: Depends on periphore-net + periphore-config. Build order: after periphore-net, before periphored.
//! D-05: Discovered peers are passive -- no auto-connect. Daemon reads the list on IPC request.

mod error;
mod list;
mod mdns;
mod probe;

pub use error::DiscoveryError;
pub use list::{DiscoveredPeerList, DiscoverySource};

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// One-way notifications from the discovery subsystem to the daemon's select! loop.
///
/// Mirrors the `PeerEvent` pattern from `periphore-net::ConnectionManager`.
#[derive(Debug)]
pub enum DiscoveryEvent {
    /// A new peer was discovered (or an existing one refreshed its last_seen time).
    PeerDiscovered {
        hostname: String,
        port: u16,
        source: DiscoverySource,
    },
    /// A peer was removed from the discovered list (mDNS goodbye or TTL expiry).
    PeerRemoved {
        hostname: String,
        /// Port is 0 for mDNS goodbye events (ServiceRemoved does not carry port, Pitfall 2).
        port: u16,
    },
    /// A non-fatal discovery error (e.g., mDNS registration failure).
    Error(String),
}

/// Manages peer discovery via mDNS and SSH tunnel port probing.
///
/// Holds the shared `DiscoveredPeerList` and spawns discovery tasks into the daemon's
/// `JoinSet`. Discovery is passive (D-05): the service only maintains the list;
/// the daemon queries it on `IpcCommand::GetDiscoveredPeers`.
pub struct DiscoveryService {
    peers: Arc<std::sync::Mutex<DiscoveredPeerList>>,
}

impl DiscoveryService {
    /// Create a new `DiscoveryService` with an empty discovered peer list.
    pub fn new() -> Self {
        Self {
            peers: Arc::new(std::sync::Mutex::new(DiscoveredPeerList::new())),
        }
    }

    /// Start discovery tasks according to `config`.
    ///
    /// Spawns into `tasks`:
    /// - mDNS register + browse task (if `config.enabled`)
    /// - SSH tunnel port probe task (if `config.ssh_probe_enabled`)
    /// - GC task (always — sweeps stale entries every 30 seconds, D-07/D-08)
    ///
    /// Tasks run until `cancel` is triggered (daemon shutdown).
    pub fn start(
        &self,
        tasks: &mut tokio::task::JoinSet<anyhow::Result<()>>,
        config: &periphore_config::DiscoveryConfig,
        event_tx: mpsc::Sender<DiscoveryEvent>,
        identity: Arc<periphore_identity::IdentityStore>,
        cancel: CancellationToken,
    ) {
        if config.enabled {
            let instance_name = config
                .instance_name
                .clone()
                .unwrap_or_else(|| "periphore".to_owned());
            let port = periphore_net::DEFAULT_PORT;
            let peers = Arc::clone(&self.peers);
            let event_tx_clone = event_tx.clone();
            let cancel_clone = cancel.clone();
            let service_type = config.service_type.clone();

            tasks.spawn(mdns::mdns_register_and_browse(
                service_type,
                instance_name,
                port,
                peers,
                event_tx_clone,
                cancel_clone,
            ));

            tracing::info!("mDNS discovery enabled");
        }

        if config.ssh_probe_enabled {
            let ports = config.ssh_probe_ports.clone();
            let own_fingerprint = identity.fingerprint;
            let peers = Arc::clone(&self.peers);
            let event_tx_clone = event_tx.clone();
            let cancel_clone = cancel.clone();

            tasks.spawn(probe::ssh_probe_loop(
                ports,
                own_fingerprint,
                identity,
                peers,
                event_tx_clone,
                cancel_clone,
            ));

            tracing::info!("SSH tunnel port probing enabled");
        }

        // Always spawn the GC task — sweeps stale entries every 30 seconds (D-07/D-08).
        {
            let peers = Arc::clone(&self.peers);
            let cancel_clone = cancel.clone();

            tasks.spawn(async move {
                loop {
                    tokio::select! {
                        _ = cancel_clone.cancelled() => {
                            tracing::debug!("discovery GC task cancelled");
                            break;
                        }
                        _ = tokio::time::sleep(Duration::from_secs(30)) => {}
                    }
                    let removed = peers
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .gc();
                    if removed > 0 {
                        tracing::debug!(
                            removed,
                            "discovery GC: removed stale peer entries (TTL expired)"
                        );
                    }
                }
                Ok(())
            });
        }
    }

    /// Return a snapshot of all currently discovered peers for IPC dispatch.
    ///
    /// Called by the daemon on `IpcCommand::GetDiscoveredPeers`.
    /// Returns `Vec<DiscoveredPeerInfo>` with `last_seen_epoch` converted from
    /// internal `Instant` to Unix epoch seconds (Pitfall 6 mitigation).
    pub fn discovered_list(&self) -> Vec<periphore_protocol::DiscoveredPeerInfo> {
        let guard = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        guard.snapshot()
    }
}

impl Default for DiscoveryService {
    fn default() -> Self {
        Self::new()
    }
}
