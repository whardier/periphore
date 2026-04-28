//! periphore-cli: CLI support library for the `periphore` binary.
//!
//! Provides command dispatch, IPC client logic, and output formatting.
//! All subcommand implementations live here; `crates/periphore/src/main.rs`
//! is a thin entry point that calls [`run`].

pub mod cli;
pub mod client;
mod commands;

pub use cli::Cli;

/// Main dispatch function for the `periphore` CLI.
///
/// Resolves the IPC socket path and dispatches to the correct subcommand handler.
///
/// # Errors
///
/// Returns an error if the daemon is not running, the IPC call fails, or
/// the command handler encounters an error.
pub async fn run(cli: Cli) -> anyhow::Result<()> {
    let socket_path = resolve_socket_path(&cli)?;
    match cli.command {
        cli::Commands::Status   => commands::status::run(&socket_path).await,
        cli::Commands::Topology => commands::topology::run(&socket_path).await,
        cli::Commands::Trust { action } => match action {
            cli::TrustAction::Accept { fingerprint } => {
                commands::trust::run_accept(&socket_path, &fingerprint).await
            }
        },
        cli::Commands::Peers { action } => match action {
            cli::PeersAction::Discovered => commands::peers::discovered::run(&socket_path).await,
            cli::PeersAction::Pending => commands::peers::pending::run(&socket_path).await,
        },
    }
}

/// Resolve the IPC socket path with priority: --socket > config.daemon.socket_path > platform default.
///
/// Config load failures are silently ignored so `periphore` works without a
/// config file, consistent with the daemon's first-run behavior.
fn resolve_socket_path(cli: &Cli) -> anyhow::Result<std::path::PathBuf> {
    if let Some(path) = &cli.socket {
        return Ok(path.clone());
    }
    if let Ok(config) = periphore_config::load(cli.config.as_deref()) {
        if let Some(path) = config.daemon.socket_path {
            return Ok(path);
        }
    }
    Ok(periphore_ipc::path::socket_path())
}
