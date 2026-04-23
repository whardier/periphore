//! periphore-cli: CLI support library for the `periphore` binary.
//!
//! Provides command dispatch, IPC client logic, and output formatting for
//! the `periphore` CLI tool. All subcommand implementations live here;
//! `crates/periphore/src/main.rs` is a thin entry point that calls into this crate.
//!
//! Full implementation: Phase 5. See ROADMAP.md Phase 5: CLI Tool.

/// Placeholder for the main CLI dispatch function.
/// Phase 5 replaces this with real subcommand handling over IPC.
///
/// # Errors
/// Returns an error if IPC connection fails or the daemon is not running.
pub fn run() -> anyhow::Result<()> {
    anyhow::bail!("periphore-cli: not yet implemented (Phase 5)")
}
