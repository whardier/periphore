//! periphore-identity: Ed25519 keypair lifecycle, SHA-256 fingerprints,
//! identicon rendering (Drunken Bishop), and BIP39 word phrases.
//!
//! SEC-01: load_or_create() — persistent Ed25519 identity.
//! SEC-02: identicon() — OpenSSH Drunken Bishop 17×9 (plan 02-02).
//! SEC-03: word_phrase() — 6 BIP39 words from fingerprint (plan 02-02).

mod bip39;

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use ed25519_dalek::SigningKey;
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Errors from identity operations.
#[derive(Debug, Error)]
pub enum IdentityError {
    /// Key file byte count is wrong. Correct is 32 (raw Ed25519 seed).
    #[error("key file is corrupt (expected 32 bytes, got {0})")]
    CorruptKeyFile(usize),
    /// Underlying I/O error (file create, read, permissions).
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    /// No XDG data directory could be determined for this user.
    #[error("no valid data directory found for this user")]
    NoDataDir,
}

/// Cryptographic identity for a Periphore node.
///
/// Holds the Ed25519 signing key and the SHA-256 fingerprint of the public key.
/// Constructed via `load_or_create` — do not construct directly.
#[derive(Debug)]
pub struct IdentityStore {
    /// The Ed25519 signing key (includes the private seed and public key).
    pub keypair: SigningKey,
    /// SHA-256 hash of the public key bytes (32 bytes). The canonical fingerprint.
    pub fingerprint: [u8; 32],
}

impl IdentityStore {
    /// Load an existing identity from `path`, or generate and persist a new one.
    ///
    /// First run: creates parent directories, generates a new Ed25519 keypair from
    /// `OsRng`, writes the 32-byte seed to `path` with mode `0600` (atomic — no
    /// world-readable race window), and logs the fingerprint at INFO level.
    ///
    /// Subsequent runs: reads the 32-byte seed, reconstructs the `SigningKey`,
    /// and derives the fingerprint. Wrong file length → `IdentityError::CorruptKeyFile`.
    ///
    /// Path resolution (the XDG data dir) is the caller's responsibility.
    /// Use `default_key_path()` for the platform-appropriate path.
    pub fn load_or_create(path: &Path) -> Result<Self, IdentityError> {
        if path.exists() {
            // --- Load existing key ---
            let bytes = std::fs::read(path)?;
            if bytes.len() != 32 {
                return Err(IdentityError::CorruptKeyFile(bytes.len()));
            }
            let seed: [u8; 32] = bytes.try_into().expect("length already validated");
            let keypair = SigningKey::from_bytes(&seed);
            let fingerprint = Self::compute_fingerprint(&keypair);
            Ok(Self { keypair, fingerprint })
        } else {
            // --- Generate new keypair (first run) ---
            // Create parent directory if it does not exist (D-04).
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let keypair = SigningKey::generate(&mut OsRng);
            let seed: [u8; 32] = keypair.to_bytes();

            // Write with mode 0600 atomically using OpenOptionsExt (RESEARCH.md Pitfall 6).
            // This eliminates the world-readable race window present in the
            // write-then-set-permissions approach.
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(path)?;
                file.write_all(&seed)?;
            }
            #[cfg(not(unix))]
            {
                std::fs::write(path, seed)?;
            }

            let fingerprint = Self::compute_fingerprint(&keypair);
            let fingerprint_hex: String =
                fingerprint.iter().map(|b| format!("{b:02x}")).collect();

            // D-15: single info log on first-run identity generation.
            tracing::info!("Generated new identity: {}", fingerprint_hex);

            Ok(Self { keypair, fingerprint })
        }
    }

    /// Return the SHA-256 fingerprint of the public key as a 64-character lowercase hex string.
    ///
    /// Output is deterministic: same key seed always produces the same string on all platforms.
    pub fn fingerprint_hex(&self) -> String {
        self.fingerprint.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Return the OpenSSH Drunken Bishop identicon for this identity's fingerprint.
    ///
    /// Pre-rendered 11-line terminal string (header + 9 grid rows + footer), newline-terminated.
    /// Implemented in plan 02-02. Returns empty string until then.
    pub fn identicon(&self) -> String {
        // TODO plan 02-02: implement Drunken Bishop algorithm (SEC-02, D-05, D-06, D-07).
        String::new()
    }

    /// Return the BIP39 word phrase for this identity's fingerprint.
    ///
    /// Returns 6 words derived from sequential 11-bit windows of the SHA-256 fingerprint.
    /// Implemented in plan 02-02. Returns empty vec until then.
    pub fn word_phrase(&self) -> Vec<String> {
        // TODO plan 02-02: implement BIP39 extraction (SEC-03, D-11, D-12).
        Vec::new()
    }

    fn compute_fingerprint(keypair: &SigningKey) -> [u8; 32] {
        let pubkey_bytes = keypair.verifying_key().to_bytes();
        Sha256::digest(pubkey_bytes).into()
    }
}

/// Return the platform-appropriate path for the identity key file.
///
/// Linux:  `$XDG_DATA_HOME/periphore/key`  (default: `~/.local/share/periphore/key`)
/// macOS:  `~/Library/Application Support/periphore/key`
///
/// Returns `None` when `ProjectDirs::from` cannot find a home directory
/// (e.g., containers running as root with no home). In that case the caller
/// should return `IdentityError::NoDataDir`.
pub fn default_key_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "periphore")
        .map(|dirs| dirs.data_dir().join("key"))
}
