//! Subcommand handlers for the `periphore` CLI.
//!
//! Each module exposes a single `run(socket_path: &Path) -> anyhow::Result<()>` function.

pub(crate) mod status;
pub(crate) mod topology;
