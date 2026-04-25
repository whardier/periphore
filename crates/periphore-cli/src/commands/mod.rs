//! Subcommand handlers for the `periphore` CLI.
//!
//! Each module exposes a single `run(socket_path: &Path) -> anyhow::Result<()>` function.

pub mod status;
pub mod topology;
