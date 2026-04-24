//! periphore-trust: fingerprint trust cache persistence and conflict detection.
//!
//! SEC-05: TrustStore — cached fingerprint persistence across sessions.
//! SEC-06: check_peer_fingerprint — hard-config fingerprint conflict detection.

pub mod store;
pub use store::{TrustError, TrustStore, TrustedPeer, check_peer_fingerprint};

use std::path::PathBuf;
use directories::ProjectDirs;

/// Return the platform-appropriate path for the trust cache file.
///
/// Linux:  `$XDG_DATA_HOME/periphore/trusted.toml`  (default: `~/.local/share/periphore/trusted.toml`)
/// macOS:  `~/Library/Application Support/periphore/trusted.toml`
///
/// Returns `None` when `ProjectDirs::from` cannot find a home directory.
pub fn default_trust_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "periphore")
        .map(|dirs| dirs.data_dir().join("trusted.toml"))
}
