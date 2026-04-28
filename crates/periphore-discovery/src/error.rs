//! periphore-discovery error types.

use thiserror::Error;

/// Errors from peer discovery operations.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// mDNS daemon failed to initialize.
    #[error("mDNS init error: {0}")]
    MdnsInit(String),

    /// mDNS browse registration failed.
    #[error("mDNS browse error: {0}")]
    MdnsBrowse(String),

    /// mDNS service registration failed.
    #[error("mDNS register error: {0}")]
    MdnsRegister(String),

    /// Underlying I/O error (SSH probe, network).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Internal error unrelated to the discovery protocol.
    #[error("internal error: {0}")]
    Internal(String),
}
