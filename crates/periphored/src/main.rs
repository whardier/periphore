use clap::Parser;
use tokio::sync::mpsc;

use periphore_ipc::IpcCommand;
use periphore_protocol::IpcResponse;

/// Periphore input sharing daemon.
///
/// Starts the IPC socket at the platform-appropriate path, loads configuration,
/// and handles input sharing between peers. Run `periphore` (the CLI tool) to
/// interact with a running daemon.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the configuration file.
    /// If not provided, defaults and environment variables are used.
    #[arg(short, long, value_name = "FILE")]
    config: Option<std::path::PathBuf>,

    /// Enable verbose (debug) logging. Overrides PERIPHORE_LOGGING_LEVEL.
    #[arg(short, long)]
    verbose: bool,
}

/// Return the identicon string for display, or an empty string when disabled.
///
/// Extracted as a free function so that SEC-04 gating logic is unit-testable
/// without running the full async daemon.
fn resolve_identicon(show_identicon: bool, identity: &periphore_identity::IdentityStore) -> String {
    if show_identicon {
        identity.identicon()
    } else {
        String::new()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // NET-06, D-15, D-16: On macOS, refuse to start if stdin is not a TTY.
    // This prevents remote SSH launch of periphored on macOS where CGEvent
    // capture requires a local graphical session.
    // macOS-only: Linux allows SSH launch as the valid remote management path.
    #[cfg(target_os = "macos")]
    {
        use std::io::IsTerminal as _;
        if !std::io::stdin().is_terminal() {
            eprintln!(
                "error: periphored must be launched from a local terminal or launchd on macOS.\n\
                 Remote SSH launch is not supported on macOS.\n\
                 Start the daemon locally, then connect to it via SSH tunnel if needed."
            );
            std::process::exit(1);
        }
    }

    let args = Args::parse();

    // -- Logging initialization --
    // Only the daemon binary initializes the tracing subscriber (D-26).
    // Library crates use tracing:: macros but never initialize a subscriber.
    // Use reload::Layer so the log filter can be updated at runtime (D-03).
    use tracing_subscriber::layer::SubscriberExt as _;
    use tracing_subscriber::util::SubscriberInitExt as _;
    let log_level = if args.verbose { "debug" } else { "info" };
    let initial_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));
    let (filter_layer, filter_handle) = tracing_subscriber::reload::Layer::new(initial_filter);
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer())
        .init();

    // -- Config loading --
    // periphore-config never writes to disk (CFG-01). CLI arg override (highest priority)
    // is handled by passing config_path here; full CLI override struct is a Phase 5 concern.
    let mut config = periphore_config::load(args.config.as_deref())
        .map_err(|e| anyhow::anyhow!("failed to load config: {e}"))?;

    tracing::info!(
        log_level = %config.logging.level,
        "periphored starting"
    );

    // -- Identity load (SEC-01) --
    // Loads the persistent Ed25519 keypair from the XDG data dir, or generates
    // a new one on first run. The key file is created with mode 0600.
    // Wrapped in Arc so it can be shared with connection tasks (Phase 6).
    let key_path = periphore_identity::default_key_path()
        .ok_or_else(|| anyhow::anyhow!("cannot determine identity key storage path"))?;
    let identity = std::sync::Arc::new(
        periphore_identity::IdentityStore::load_or_create(&key_path)
            .map_err(|e| anyhow::anyhow!("identity error: {e}"))?,
    );
    tracing::info!(
        fingerprint = %identity.fingerprint_hex(),
        "identity loaded"
    );

    // -- Trust cache load (SEC-05) --
    // Loads the fingerprint trust cache from the XDG data dir. If the file
    // does not exist (first run), starts with an empty cache. The cache is
    // only written to via AcceptFingerprint IPC.
    // Wrapped in Arc<RwLock<>> so it can be shared with connection tasks (Phase 6).
    let trust_path = periphore_trust::default_trust_path()
        .ok_or_else(|| anyhow::anyhow!("cannot determine trust cache storage path"))?;
    let trust_store = std::sync::Arc::new(std::sync::RwLock::new(
        periphore_trust::TrustStore::load(&trust_path)
            .map_err(|e| anyhow::anyhow!("trust cache error: {e}"))?,
    ));
    tracing::info!(path = %trust_path.display(), "trust cache loaded");

    // -- IPC socket path --
    // Use daemon.socket_path from config if set; otherwise use platform default.
    let socket_path = config
        .daemon
        .socket_path
        .clone()
        .unwrap_or_else(periphore_ipc::path::socket_path);

    tracing::info!(path = %socket_path.display(), "IPC socket path");

    // -- Signal handlers (D-29) --
    // Must be registered before spawning tasks to ensure signals are not missed.
    #[cfg(unix)]
    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    #[cfg(unix)]
    let mut sighup =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())?;

    // -- IPC channel --
    // Bounded channel: 64 slots. IPC owns transport; daemon owns routing.
    // Back-pressure prevents unbounded memory growth if the router is slow.
    let (ipc_cmd_tx, mut ipc_cmd_rx) = mpsc::channel::<IpcCommand>(64);

    // -- Spawn tasks --
    let mut tasks = tokio::task::JoinSet::new();

    // -- Phase 6: ConnectionManager for TCP peering (NET-01, NET-03) --
    let (net_event_tx, mut net_event_rx) = tokio::sync::mpsc::channel::<periphore_net::PeerEvent>(64);
    let mut conn_mgr = periphore_net::ConnectionManager::new(net_event_tx);
    let mut focus_sm = periphore_core::FocusStateMachine::new();

    // Spawn listener if daemon.listen = true (D-07)
    if config.daemon.listen {
        let port = config.daemon.port.unwrap_or(periphore_net::DEFAULT_PORT);
        let bind_addr: std::net::SocketAddr = format!("0.0.0.0:{port}")
            .parse()
            .expect("valid socket address");
        conn_mgr.spawn_listener(
            &mut tasks,
            bind_addr,
            std::sync::Arc::clone(&identity),
            std::sync::Arc::clone(&trust_store),
        );
        tracing::info!(port, "TCP listener started");
    }

    // Spawn outbound connectors for all configured peers with a host (D-05)
    for peer in &config.peers {
        if peer.host.is_some() {
            conn_mgr.spawn_connector(
                &mut tasks,
                peer.clone(),
                std::sync::Arc::clone(&identity),
                std::sync::Arc::clone(&trust_store),
            );
            tracing::info!(
                name = peer.name.as_deref().unwrap_or("(unnamed)"),
                host = peer.host.as_deref().unwrap_or(""),
                "outbound connector started"
            );
        }
    }

    let ipc_path = socket_path.clone();
    tasks.spawn(async move {
        periphore_ipc::serve(&ipc_path, ipc_cmd_tx)
            .await
            .map_err(|e| anyhow::anyhow!("IPC server error: {e}"))
    });

    tracing::info!("periphored running -- waiting for signals or IPC commands");

    // -- Main event loop --
    // tokio::select! polls all branches concurrently. First ready branch runs.
    loop {
        tokio::select! {
            // Signal: SIGTERM -- clean shutdown
            _ = sigterm.recv() => {
                tracing::info!("SIGTERM received -- shutting down");
                break;
            }

            // Signal: SIGHUP -- config reload (D-01, D-02, D-03)
            _ = sighup.recv() => {
                tracing::info!("SIGHUP received -- reloading config");
                if let Some(new_config) = reload_config(
                    args.config.as_deref(),
                    &config,
                    &filter_handle,
                ) {
                    // D-11: cancel reconnect tasks for peers removed from config
                    let old_peer_keys: std::collections::HashSet<String> = config.peers.iter()
                        .filter_map(|p| p.host.as_ref().map(|h| {
                            format!("{}:{}", h, p.port.unwrap_or(periphore_net::DEFAULT_PORT))
                        }))
                        .collect();
                    let new_peer_keys: std::collections::HashSet<String> = new_config.peers.iter()
                        .filter_map(|p| p.host.as_ref().map(|h| {
                            format!("{}:{}", h, p.port.unwrap_or(periphore_net::DEFAULT_PORT))
                        }))
                        .collect();
                    for removed_key in old_peer_keys.difference(&new_peer_keys) {
                        conn_mgr.cancel_peer(removed_key);
                        tracing::info!(peer = %removed_key, "peer removed from config -- reconnect cancelled");
                    }
                    config = new_config;
                }
            }

            // TCP peer event from ConnectionManager
            net_event = net_event_rx.recv() => {
                match net_event {
                    Some(periphore_net::PeerEvent::PeerPending { fingerprint, identicon, word_phrase }) => {
                        // D-02: log at WARN so operator sees the pending peer in daemon output
                        tracing::warn!(
                            fingerprint = %fingerprint,
                            "unknown peer pending verification -- run: periphore trust accept {}",
                            &fingerprint[..fingerprint.len().min(16)]
                        );
                        if !identicon.is_empty() {
                            tracing::warn!("peer identicon:\n{identicon}");
                        }
                        if !word_phrase.is_empty() {
                            tracing::warn!(phrase = %word_phrase.join(" "), "peer word phrase");
                        }
                    }
                    Some(periphore_net::PeerEvent::PeerConnected { peer_id }) => {
                        tracing::info!(peer_id = %peer_id, "peer connected and trusted");
                    }
                    Some(periphore_net::PeerEvent::PeerDisconnected { peer_id }) => {
                        tracing::info!(peer_id = %peer_id, "peer disconnected");
                        // Return focus if we were forwarding to this peer
                        let _ = focus_sm.reclaim();
                    }
                    None => {
                        tracing::warn!("net event channel closed");
                    }
                }
            }

            // IPC command from client
            cmd = ipc_cmd_rx.recv() => {
                match cmd {
                    Some(IpcCommand::GetStatus { responder }) => {
                        tracing::debug!("IPC: GetStatus");
                        let _ = responder.send(IpcResponse::Status {
                            running:     true,
                            fingerprint: Some(identity.fingerprint_hex()),
                        });
                    }
                    Some(IpcCommand::InjectInputEvent { event, responder }) => {
                        // D-19: InjectInputEvent is the IPC test backbone.
                        // Phase 9 wires this to real capture/inject; for now, log and ack.
                        tracing::debug!(?event, "IPC: InjectInputEvent");
                        let _ = responder.send(IpcResponse::Ok);
                    }
                    Some(IpcCommand::SimulateEdgeCross { edge, position, responder }) => {
                        // Phase 6: FocusStateMachine wired (D-21). Phase 8 adds real topology routing.
                        tracing::debug!(?edge, position, "IPC: SimulateEdgeCross -- focus state: {:?}", focus_sm.current_state());
                        let _ = responder.send(IpcResponse::Ok);
                    }
                    Some(IpcCommand::GetIdenticon { responder, .. }) => {
                        tracing::debug!("IPC: GetIdenticon");
                        let _ = responder.send(IpcResponse::Identicon {
                            fingerprint_hex: identity.fingerprint_hex(),
                            identicon:       resolve_identicon(config.identity.show_identicon, &identity),
                        });
                    }
                    Some(IpcCommand::GetWordPhrase { responder, .. }) => {
                        tracing::debug!("IPC: GetWordPhrase");
                        let words = identity.word_phrase();
                        let phrase = words.join(" ");
                        let _ = responder.send(IpcResponse::WordPhrase { words, phrase });
                    }
                    Some(IpcCommand::AcceptFingerprint { fingerprint, responder }) => {
                        tracing::info!(fingerprint = %fingerprint, "IPC: AcceptFingerprint");
                        match trust_store.write().unwrap().add_trusted(&fingerprint, None, &trust_path) {
                            Ok(()) => {
                                tracing::info!(fingerprint = %fingerprint, "fingerprint trusted and cached");
                                // Phase 6: promote any pending connection with this fingerprint
                                let _ = conn_mgr.promote_pending(&fingerprint).await;
                                let _ = responder.send(IpcResponse::Ok);
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "failed to cache trusted fingerprint");
                                let _ = responder.send(IpcResponse::Error {
                                    message: format!("trust cache error: {e}"),
                                });
                            }
                        }
                    }
                    Some(IpcCommand::RejectFingerprint { fingerprint, responder }) => {
                        // Rejection is stateless — no state change needed.
                        // The daemon simply does not add the fingerprint to the trust cache.
                        tracing::info!(fingerprint = %fingerprint, "IPC: RejectFingerprint (no state change)");
                        let _ = responder.send(IpcResponse::Ok);
                    }
                    Some(IpcCommand::GetPendingVerifications { responder }) => {
                        tracing::debug!("IPC: GetPendingVerifications");
                        let peers = conn_mgr.pending_list();
                        let _ = responder.send(IpcResponse::PendingPeers { peers });
                    }
                    Some(IpcCommand::ReloadConfig { responder }) => {
                        tracing::info!("IPC: ReloadConfig");
                        match reload_config(
                            args.config.as_deref(),
                            &config,
                            &filter_handle,
                        ) {
                            Some(new_config) => {
                                // D-11: cancel reconnect tasks for peers removed from config
                                let old_peer_keys: std::collections::HashSet<String> = config.peers.iter()
                                    .filter_map(|p| p.host.as_ref().map(|h| {
                                        format!("{}:{}", h, p.port.unwrap_or(periphore_net::DEFAULT_PORT))
                                    }))
                                    .collect();
                                let new_peer_keys: std::collections::HashSet<String> = new_config.peers.iter()
                                    .filter_map(|p| p.host.as_ref().map(|h| {
                                        format!("{}:{}", h, p.port.unwrap_or(periphore_net::DEFAULT_PORT))
                                    }))
                                    .collect();
                                for removed_key in old_peer_keys.difference(&new_peer_keys) {
                                    conn_mgr.cancel_peer(removed_key);
                                    tracing::info!(peer = %removed_key, "peer removed from config -- reconnect cancelled");
                                }
                                config = new_config;
                                let _ = responder.send(IpcResponse::Ok);
                            }
                            None => {
                                let _ = responder.send(IpcResponse::Error {
                                    message: "config reload failed -- see daemon logs".to_owned(),
                                });
                            }
                        }
                    }
                    Some(other) => {
                        // All other commands: acknowledge with Ok for now.
                        // Phase-specific plans will add real dispatch here.
                        tracing::debug!("IPC: unhandled command (sending Ok)");
                        send_ok(other);
                    }
                    None => {
                        // Channel closed -- all senders dropped. Shutdown.
                        tracing::warn!("IPC command channel closed unexpectedly -- shutting down");
                        break;
                    }
                }
            }

            // Task completion: handle IPC server task exit
            result = tasks.join_next(), if !tasks.is_empty() => {
                match result {
                    Some(Ok(Ok(()))) => {
                        tracing::info!("IPC server task completed -- shutting down");
                        break;
                    }
                    Some(Ok(Err(e))) => {
                        tracing::error!("IPC server task error: {e}");
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::error!("Task panicked: {e}");
                        break;
                    }
                    None => {
                        // JoinSet empty -- unreachable with is_empty guard, but handled defensively.
                    }
                }
            }
        }
    }

    // -- Clean shutdown --
    // Cancel all spawned tasks.
    tasks.abort_all();

    // Remove IPC socket (D-18, D-29). .ok() suppresses error if already removed.
    let _ = std::fs::remove_file(&socket_path);

    tracing::info!("periphored shutdown complete");
    Ok(())
}

/// Reload configuration from disk and update live daemon state.
///
/// On success: hot-reloads the tracing filter if `logging.level` changed (D-03),
/// logs warnings for restart-required fields (D-02), and returns the new Config.
///
/// On failure: logs the error and returns `None` — the caller keeps the existing
/// config (D-04). The daemon never crashes on reload failure.
///
/// Identity and trust store are NOT reloaded here (D-05).
fn reload_config<S>(
    config_path: Option<&std::path::Path>,
    current_config: &periphore_config::Config,
    filter_handle: &tracing_subscriber::reload::Handle<tracing_subscriber::EnvFilter, S>,
) -> Option<periphore_config::Config>
where
    S: tracing::Subscriber,
{
    match periphore_config::load(config_path) {
        Ok(new_config) => {
            // Hot-reload logging level (D-03)
            if new_config.logging.level != current_config.logging.level {
                match tracing_subscriber::EnvFilter::try_new(&new_config.logging.level) {
                    Ok(new_filter) => {
                        if let Err(e) = filter_handle.reload(new_filter) {
                            tracing::warn!(error = %e, "failed to reload tracing filter");
                        } else {
                            tracing::info!(
                                level = %new_config.logging.level,
                                "logging level reloaded"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "invalid logging level in new config -- keeping old filter"
                        );
                    }
                }
            }

            // Restart-required fields: log warn if changed, do NOT apply (D-02)
            if new_config.daemon.socket_path != current_config.daemon.socket_path {
                tracing::warn!(
                    "config field 'daemon.socket_path' changed but requires restart to take effect"
                );
            }
            if new_config.daemon.port != current_config.daemon.port {
                tracing::warn!(
                    "config field 'daemon.port' changed but requires restart to take effect"
                );
            }
            if new_config.daemon.listen != current_config.daemon.listen {
                tracing::warn!(
                    "config field 'daemon.listen' changed but requires restart to take effect"
                );
            }

            tracing::info!("config reloaded successfully");
            Some(new_config)
        }
        Err(e) => {
            tracing::error!(error = %e, "config reload failed -- keeping existing config");
            None
        }
    }
}

/// Send `IpcResponse::Ok` (or appropriate placeholder) to remaining `IpcCommand` variants.
/// Used for commands that don't have real dispatch in Phase 1.
fn send_ok(cmd: IpcCommand) {
    match cmd {
        IpcCommand::ListPeers { responder } => {
            let _ = responder.send(IpcResponse::Peers { peers: vec![] });
        }
        IpcCommand::GetTopology { responder } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::GetState { responder } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        // GetStatus, InjectInputEvent, SimulateEdgeCross, ReloadConfig,
        // GetIdenticon, GetWordPhrase, AcceptFingerprint, RejectFingerprint,
        // GetPendingVerifications have dedicated arms in the main select! loop
        // and never reach send_ok.
        // The wildcard arm satisfies Rust's exhaustiveness requirement without
        // duplicating response logic that already exists in the select! arms.
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_identicon;
    use periphore_identity::IdentityStore;
    use std::fs;

    const TEST_SEED: [u8; 32] = [0u8; 32];

    fn make_test_identity() -> (IdentityStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("temp dir");
        let key_path = dir.path().join("key");
        fs::write(&key_path, TEST_SEED).expect("write test seed");
        let store = IdentityStore::load_or_create(&key_path)
            .expect("load from known seed");
        (store, dir) // dir kept alive to prevent early cleanup
    }

    #[test]
    fn test_show_identicon_suppressed_when_disabled() {
        // SEC-04: when show_identicon is false the identicon field must be empty.
        let (identity, _dir) = make_test_identity();
        let result = resolve_identicon(false, &identity);
        assert!(
            result.is_empty(),
            "identicon must be empty string when show_identicon=false, got: {result:?}"
        );
    }

    #[test]
    fn test_show_identicon_returned_when_enabled() {
        // SEC-04: when show_identicon is true (default) the identicon is returned normally.
        let (identity, _dir) = make_test_identity();
        let result = resolve_identicon(true, &identity);
        assert!(
            !result.is_empty(),
            "identicon must be non-empty when show_identicon=true"
        );
        // Must still be a valid 11-line Drunken Bishop string.
        assert_eq!(
            result.lines().count(),
            11,
            "identicon must have 11 lines when enabled, got {}",
            result.lines().count()
        );
    }
}
