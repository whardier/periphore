//! CLI argument surface for the `periphore` binary.
//!
//! Defines [`Cli`] (global args + subcommand) and [`Commands`] (subcommand enum).
//! Parsing (`Cli::parse()`) happens in `crates/periphore/src/main.rs`.

use clap::{Parser, Subcommand};

/// Periphore input sharing CLI.
///
/// Interact with a running `periphored` daemon. If the daemon is not running,
/// most commands will fail with a clear error message.
///
/// Start the daemon first: `periphored [--config FILE]`
#[derive(Parser, Debug)]
#[command(name = "periphore", version, about = "Periphore input sharing CLI", long_about = None)]
pub struct Cli {
    /// Path to a custom IPC socket (overrides platform default and config).
    #[arg(long, global = true, value_name = "PATH")]
    pub socket: Option<std::path::PathBuf>,

    /// Path to the configuration file (for socket_path override lookup).
    #[arg(short, long, global = true, value_name = "FILE")]
    pub config: Option<std::path::PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands for `periphore`.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Connect to the daemon and report its status and identity fingerprint.
    Status,
    /// Show the resolved monitor topology (requires daemon; stub output until Phase 8).
    Topology,
    /// Manage peer trust (fingerprint acceptance).
    Trust {
        #[command(subcommand)]
        action: TrustAction,
    },
    /// Manage and inspect peers.
    Peers {
        #[command(subcommand)]
        action: PeersAction,
    },
}

/// Sub-actions for `periphore trust`.
#[derive(Subcommand, Debug)]
pub enum TrustAction {
    /// Accept a peer's fingerprint and add it to the trust cache.
    ///
    /// The fingerprint is the 64-character hex string shown in daemon logs:
    ///   WARN unknown peer pending verification -- run: periphore trust accept <fingerprint>
    Accept {
        /// 64-character hex fingerprint to trust.
        fingerprint: String,
    },
}

/// Sub-actions for `periphore peers`.
#[derive(Subcommand, Debug)]
pub enum PeersAction {
    /// List peers discovered via mDNS or SSH tunnel probe.
    Discovered,
    /// List peers awaiting trust verification.
    Pending,
}
