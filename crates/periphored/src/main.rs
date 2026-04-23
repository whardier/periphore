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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // -- Logging initialization --
    // Only the daemon binary initializes the tracing subscriber (D-26).
    // Library crates use tracing:: macros but never initialize a subscriber.
    let log_level = if args.verbose { "debug" } else { "info" };
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set tracing subscriber");

    // -- Config loading --
    // periphore-config never writes to disk (CFG-01). CLI arg override (highest priority)
    // is handled by passing config_path here; full CLI override struct is a Phase 5 concern.
    let config = periphore_config::load(args.config.as_deref())
        .map_err(|e| anyhow::anyhow!("failed to load config: {e}"))?;

    tracing::info!(
        log_level = %config.logging.level,
        "periphored starting"
    );

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

            // Signal: SIGHUP -- config reload (placeholder; full reload in Phase 4)
            _ = sighup.recv() => {
                tracing::info!("SIGHUP received -- config reload not yet implemented (Phase 4)");
                // TODO Phase 4: reload config from disk and update live state.
            }

            // IPC command from client
            cmd = ipc_cmd_rx.recv() => {
                match cmd {
                    Some(IpcCommand::GetStatus { responder }) => {
                        // D-27: Respond to GetStatus with running=true and no fingerprint.
                        // Phase 2 will fill in the real identity fingerprint.
                        tracing::debug!("IPC: GetStatus");
                        let _ = responder.send(IpcResponse::Status {
                            running:     true,
                            fingerprint: None, // Phase 2: real Ed25519 fingerprint
                        });
                    }
                    Some(IpcCommand::InjectInputEvent { event, responder }) => {
                        // D-19: InjectInputEvent is the IPC test backbone.
                        // Phase 9 wires this to real capture/inject; for now, log and ack.
                        tracing::debug!(?event, "IPC: InjectInputEvent");
                        let _ = responder.send(IpcResponse::Ok);
                    }
                    Some(IpcCommand::SimulateEdgeCross { edge, position, responder }) => {
                        // D-19: SimulateEdgeCross is the IPC test backbone.
                        // Phase 8 wires this to real topology; for now, log and ack.
                        tracing::debug!(?edge, position, "IPC: SimulateEdgeCross");
                        let _ = responder.send(IpcResponse::Ok);
                    }
                    Some(IpcCommand::ReloadConfig { responder }) => {
                        tracing::info!("IPC: ReloadConfig (Phase 4 placeholder)");
                        let _ = responder.send(IpcResponse::Ok);
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
            result = tasks.join_next() => {
                match result {
                    Some(Ok(Ok(()))) => {
                        tracing::info!("IPC server task completed");
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
                        // JoinSet empty -- no more tasks.
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
        IpcCommand::AcceptFingerprint { responder, .. } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::RejectFingerprint { responder, .. } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::GetState { responder } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::GetPendingVerifications { responder } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::GetIdenticon { responder, .. } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::GetWordPhrase { responder, .. } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        // GetStatus, InjectInputEvent, SimulateEdgeCross, and ReloadConfig have
        // dedicated arms in the main select! loop and never reach send_ok.
        // The wildcard arm satisfies Rust's exhaustiveness requirement without
        // duplicating response logic that already exists in the select! arms.
        _ => {}
    }
}
