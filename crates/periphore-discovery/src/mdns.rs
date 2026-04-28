//! mDNS service registration and browse loop.
//!
//! Uses the `mdns-sd` crate for RFC 6762/6763 compliant service discovery.
//! Spawns as a task in DiscoveryService::start().
//!
//! D-03: Only runs when DiscoveryConfig.enabled = true.
//! Pitfall 1: mDNS may fail silently on corporate networks -- daemon continues normally.
//! Pitfall 5: Always call mdns.shutdown() before dropping the receiver.

use std::sync::Arc;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::list::{DiscoveredPeerList, DiscoverySource};
use crate::DiscoveryEvent;

/// Register the local Periphore service via mDNS and browse for peers.
///
/// On mDNS daemon init failure, logs `tracing::warn!` and returns `Ok(())` — daemon
/// continues normally without discovery (CLAUDE.md item 6, Pitfall 1 mitigation).
///
/// On cancellation, shuts down the mDNS daemon before returning.
pub(crate) async fn mdns_register_and_browse(
    service_type: String,
    instance_name: String,
    port: u16,
    peers: Arc<std::sync::Mutex<DiscoveredPeerList>>,
    event_tx: mpsc::Sender<DiscoveryEvent>,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    // Initialize the mDNS daemon. On failure, warn and continue without discovery.
    let mdns = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(error = %e, "mDNS daemon failed to start — discovery unavailable on this network");
            return Ok(());
        }
    };

    // Build the service info.
    // mdns-sd requires host_name to end with ".local." (validated at register time).
    // Passing "" causes "Hostname must end with '.local.'" and falls back to browse-only.
    // We construct host_name from instance_name (already unique per IN-03 fix).
    // enable_addr_auto() detects local IP addresses on all interfaces.
    let host_name = format!("{instance_name}.local.");
    let properties = [("proto_ver", periphore_net::PROTOCOL_VERSION.to_string())];
    let service_info = match ServiceInfo::new(
        &service_type,
        &instance_name,
        &host_name,
        "",    // auto-detect IP via enable_addr_auto()
        port,
        &properties[..],
    ) {
        Ok(info) => info.enable_addr_auto(),
        Err(e) => {
            tracing::warn!(error = %e, "mDNS ServiceInfo creation failed — skipping registration");
            // Continue in browse-only mode
            browse_loop(mdns, service_type, peers, event_tx, cancel).await;
            return Ok(());
        }
    };

    // Register the local service. On failure, continue in browse-only mode.
    if let Err(e) = mdns.register(service_info) {
        tracing::warn!(
            error = %e,
            instance = %instance_name,
            "mDNS service registration failed — browsing in read-only mode"
        );
    } else {
        tracing::info!(
            service_type = %service_type,
            instance = %instance_name,
            port,
            "mDNS service registered"
        );
    }

    browse_loop(mdns, service_type, peers, event_tx, cancel).await;
    Ok(())
}

/// Run the mDNS browse loop until cancelled.
///
/// Handles ServiceResolved (upsert into peer list, emit PeerDiscovered) and
/// ServiceRemoved (remove from peer list by fullname, emit PeerRemoved).
async fn browse_loop(
    mdns: ServiceDaemon,
    service_type: String,
    peers: Arc<std::sync::Mutex<DiscoveredPeerList>>,
    event_tx: mpsc::Sender<DiscoveryEvent>,
    cancel: CancellationToken,
) {
    // Start browsing. On failure, warn and shut down gracefully.
    let receiver = match mdns.browse(&service_type) {
        Ok(rx) => rx,
        Err(e) => {
            tracing::warn!(error = %e, "mDNS browse failed — discovery unavailable");
            let _ = mdns.shutdown();
            return;
        }
    };

    tracing::info!(service_type = %service_type, "mDNS browse started");

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                tracing::debug!("mDNS browse loop cancelled — shutting down");
                let _ = mdns.shutdown();
                break;
            }
            result = receiver.recv_async() => {
                match result {
                    Ok(ServiceEvent::ServiceResolved(info)) => {
                        let hostname = info.get_hostname().trim_end_matches('.').to_owned();
                        let port = info.get_port();
                        let fullname = info.get_fullname().to_owned();

                        tracing::debug!(
                            hostname = %hostname,
                            port,
                            fullname = %fullname,
                            "mDNS peer resolved"
                        );

                        peers
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .upsert(hostname.clone(), port, DiscoverySource::Mdns, Some(fullname));

                        event_tx
                            .send(DiscoveryEvent::PeerDiscovered {
                                hostname,
                                port,
                                source: DiscoverySource::Mdns,
                            })
                            .await
                            .ok();
                    }
                    Ok(ServiceEvent::ServiceRemoved(_ty, fullname)) => {
                        tracing::debug!(fullname = %fullname, "mDNS peer removed (goodbye)");

                        // Immediate removal (D-07 hybrid expiry: goodbye fires immediate removal)
                        let removed = peers
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .remove_by_fullname(&fullname);

                        if removed {
                            // Extract instance name from fullname as hostname approximation.
                            // fullname format: "instance.service_type.local."
                            let hostname = fullname
                                .split('.')
                                .next()
                                .unwrap_or(&fullname)
                                .to_owned();
                            event_tx
                                .send(DiscoveryEvent::PeerRemoved { hostname, port: 0 })
                                .await
                                .ok();
                        }
                    }
                    Ok(ServiceEvent::SearchStarted(ty)) => {
                        tracing::trace!(service_type = %ty, "mDNS search started");
                    }
                    Ok(ServiceEvent::ServiceFound(ty, fullname)) => {
                        tracing::trace!(service_type = %ty, fullname = %fullname, "mDNS service found (unresolved)");
                    }
                    Ok(ServiceEvent::SearchStopped(ty)) => {
                        tracing::trace!(service_type = %ty, "mDNS search stopped");
                    }
                    Ok(_) => {
                        // Future ServiceEvent variants — ignore gracefully.
                    }
                    Err(_) => {
                        // Channel closed — mdns daemon shut down
                        tracing::debug!("mDNS event channel closed");
                        break;
                    }
                }
            }
        }
    }
}
