//! Trust cache store: fingerprint persistence and conflict detection.

use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from trust cache operations.
#[derive(Debug, Error)]
pub enum TrustError {
    /// Trust cache file exists but contains invalid TOML.
    #[error("trust cache file is corrupt: {0}")]
    CorruptCacheFile(String),

    /// Fingerprint conflict: configured fingerprint does not match actual.
    #[error("fingerprint conflict for peer '{peer_label}': expected {expected}, got {actual}")]
    FingerprintConflict {
        expected: String,
        actual: String,
        peer_label: String,
    },

    /// Duplicate fingerprint already in trust cache.
    #[error("fingerprint already trusted: {0}")]
    AlreadyTrusted(String),

    /// Fingerprint not found in trust cache (for remove operations).
    #[error("fingerprint not found in trust cache: {0}")]
    NotFound(String),

    /// TOML serialization error.
    #[error("serialization error: {0}")]
    SerializeError(String),

    /// Underlying I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

/// Internal TOML document structure for trusted.toml.
#[derive(Debug, Serialize, Deserialize)]
struct TrustCache {
    #[serde(default)]
    trusted: Vec<TrustedPeer>,
}

/// A single trusted peer entry in the cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedPeer {
    pub fingerprint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

/// Fingerprint trust cache backed by a TOML file.
#[derive(Debug)]
pub struct TrustStore {
    cache: TrustCache,
}

impl TrustStore {
    /// Load the trust cache from disk. Returns empty cache if file does not exist.
    pub fn load(path: &Path) -> Result<Self, TrustError> {
        if path.exists() {
            let content = std::fs::read_to_string(path).map_err(TrustError::Io)?;
            let cache: TrustCache = toml::from_str(&content)
                .map_err(|e| TrustError::CorruptCacheFile(e.to_string()))?;
            tracing::info!(path = %path.display(), count = cache.trusted.len(), "trust cache loaded");
            Ok(Self { cache })
        } else {
            tracing::info!(path = %path.display(), "trust cache file not found — starting with empty cache");
            Ok(Self {
                cache: TrustCache { trusted: vec![] },
            })
        }
    }

    /// Check if a fingerprint is in the trust cache.
    pub fn is_trusted(&self, fp: &str) -> bool {
        let fp_lower = fp.to_ascii_lowercase();
        self.cache.trusted.iter().any(|p| p.fingerprint == fp_lower)
    }

    /// Add a fingerprint to the trust cache and persist to disk.
    /// Idempotent: returns Ok(()) if fingerprint is already trusted (updates alias if provided).
    pub fn add_trusted(&mut self, fp: &str, alias: Option<&str>, path: &Path) -> Result<(), TrustError> {
        let fp_lower = fp.to_ascii_lowercase();

        // Idempotent: if already trusted, update alias if provided, then return Ok.
        if let Some(existing) = self.cache.trusted.iter_mut().find(|p| p.fingerprint == fp_lower) {
            if alias.is_some() {
                existing.alias = alias.map(String::from);
            }
            tracing::info!(fingerprint = %fp_lower, "fingerprint already trusted (idempotent)");
            return Ok(());
        }

        self.cache.trusted.push(TrustedPeer {
            fingerprint: fp_lower.clone(),
            alias: alias.map(String::from),
        });
        tracing::info!(fingerprint = %fp_lower, "fingerprint added to trust cache");
        self.save(path)
    }

    /// Remove a fingerprint from the trust cache and persist to disk.
    pub fn remove_trusted(&mut self, fp: &str, path: &Path) -> Result<(), TrustError> {
        let fp_lower = fp.to_ascii_lowercase();
        let before = self.cache.trusted.len();
        self.cache.trusted.retain(|p| p.fingerprint != fp_lower);
        if self.cache.trusted.len() == before {
            return Err(TrustError::NotFound(fp_lower));
        }
        tracing::info!(fingerprint = %fp_lower, "fingerprint removed from trust cache");
        self.save(path)
    }

    fn save(&self, path: &Path) -> Result<(), TrustError> {
        let toml_str = toml::to_string_pretty(&self.cache)
            .map_err(|e| TrustError::SerializeError(e.to_string()))?;

        // Ensure parent directory exists (first-run case).
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(TrustError::Io)?;
        }

        // NamedTempFile::new_in creates file in same dir as target (same filesystem
        // guarantees atomic rename). Default permissions are 0o600 on Unix.
        let mut tmp = tempfile::NamedTempFile::new_in(
            path.parent().unwrap_or(Path::new(".")),
        )
        .map_err(TrustError::Io)?;

        std::io::Write::write_all(&mut tmp, toml_str.as_bytes())
            .map_err(TrustError::Io)?;

        // Flush to disk before rename for durability.
        tmp.as_file().sync_all().map_err(TrustError::Io)?;

        // Atomic rename — other processes see old or new, never partial.
        tmp.persist(path).map_err(|e| TrustError::Io(e.error))?;

        tracing::debug!(path = %path.display(), "trust cache written to disk");
        Ok(())
    }
}

/// Check whether a peer's actual fingerprint matches the configured expectation.
///
/// Returns `Ok(())` if they match (case-insensitive), or
/// `Err(TrustError::FingerprintConflict)` if not.
///
/// `peer_label` provides context for error messages (name or fingerprint prefix).
/// This is a pure function with no I/O — designed for unit testing in isolation.
pub fn check_peer_fingerprint(
    configured_fp: &str,
    actual_fp: &str,
    peer_label: &str,
) -> Result<(), TrustError> {
    if configured_fp.to_ascii_lowercase() == actual_fp.to_ascii_lowercase() {
        Ok(())
    } else {
        Err(TrustError::FingerprintConflict {
            expected: configured_fp.to_owned(),
            actual: actual_fp.to_owned(),
            peer_label: peer_label.to_owned(),
        })
    }
}
