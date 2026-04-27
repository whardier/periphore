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
    /// SEC-02: visual fingerprint verification via Drunken Bishop algorithm (D-05, D-06, D-07).
    pub fn identicon(&self) -> String {
        drunken_bishop(&self.fingerprint)
    }

    /// Return the BIP39 word phrase for this identity's fingerprint.
    ///
    /// Returns 6 words derived from sequential 11-bit windows of the SHA-256 fingerprint.
    /// SEC-03: typed character-by-character verification (D-11, D-12, D-13).
    pub fn word_phrase(&self) -> Vec<String> {
        word_indices(&self.fingerprint)
            .iter()
            .map(|&i| crate::bip39::BIP39_WORDS[i].to_owned())
            .collect()
    }

    fn compute_fingerprint(keypair: &SigningKey) -> [u8; 32] {
        let pubkey_bytes = keypair.verifying_key().to_bytes();
        Sha256::digest(pubkey_bytes).into()
    }
}

/// Return the Drunken Bishop identicon for an arbitrary 32-byte fingerprint.
///
/// Used to display visual identification for PEER fingerprints during
/// pending verification (D-02). For local identity, use `IdentityStore::identicon()`.
/// Same algorithm as `IdentityStore::identicon()` — operates on the fingerprint
/// bytes directly without requiring a keypair or IdentityStore.
pub fn identicon_from_fingerprint(fp: &[u8; 32]) -> String {
    drunken_bishop(fp)
}

/// Return the BIP39 word phrase for an arbitrary 32-byte fingerprint.
///
/// Used to display verbal verification words for PEER fingerprints during
/// pending verification (D-02). For local identity, use `IdentityStore::word_phrase()`.
/// Same algorithm as `IdentityStore::word_phrase()` — operates on the fingerprint
/// bytes directly without requiring a keypair or IdentityStore.
pub fn word_phrase_from_fingerprint(fp: &[u8; 32]) -> Vec<String> {
    word_indices(fp)
        .iter()
        .map(|&i| crate::bip39::BIP39_WORDS[i].to_owned())
        .collect()
}

/// OpenSSH Drunken Bishop identicon algorithm.
///
/// Input: 32-byte fingerprint (SHA-256 of public key).
/// Output: 11-line string — header + 9 grid rows + footer — newline-terminated.
///
/// Character table: " .o+=*BOX@%&#/^SE" (17 chars, index 0-16).
/// Grid: 17 columns × 9 rows. Start: (col=8, row=4).
/// Bit order: LSB first within each byte (RESEARCH.md Pitfall 3).
/// D-05: 17×9 grid, center start.
/// D-06: header "+--[ED25519 256]--+", footer "+--[PERIPHORE]----+".
/// D-07: input is the SHA-256 fingerprint bytes.
fn drunken_bishop(fingerprint: &[u8; 32]) -> String {
    const CHARS: &[u8; 17] = b" .o+=*BOX@%&#/^SE";
    const COLS: i32 = 17;
    const ROWS: i32 = 9;

    let mut grid = [0u32; (17 * 9) as usize];
    let mut col: i32 = 8;
    let mut row: i32 = 4;

    for &byte in fingerprint.iter() {
        let mut b = byte;
        for _ in 0..4 {
            let bits = b & 0x3;
            b >>= 2;
            let dx: i32 = if bits & 0x01 != 0 { 1 } else { -1 };
            let dy: i32 = if bits & 0x02 != 0 { 1 } else { -1 };
            col = (col + dx).clamp(0, COLS - 1);
            row = (row + dy).clamp(0, ROWS - 1);
            grid[(row * COLS + col) as usize] += 1;
        }
    }

    let end_pos = (row * COLS + col) as usize;
    let start_pos = (4 * COLS + 8) as usize; // 76

    let header = build_border("ED25519 256");
    let footer = build_border("PERIPHORE");

    let mut out = String::with_capacity(11 * 20);
    out.push_str(&header);
    out.push('\n');

    for r in 0..(ROWS as usize) {
        out.push('|');
        for c in 0..(COLS as usize) {
            let pos = r * (COLS as usize) + c;
            let ch = if pos == end_pos {
                b'E'
            } else if pos == start_pos {
                b'S'
            } else {
                CHARS[grid[pos].min(16) as usize]
            };
            out.push(ch as char);
        }
        out.push('|');
        out.push('\n');
    }

    out.push_str(&footer);
    out.push('\n');
    out
}

/// Build a border line for the identicon: "+--[label]----+"
///
/// Total line length is 19 chars:
///   4 ("+--[") + label.len() + 1 ("]") + dashes + 1 ("+") = 19
///   → dashes = 13 - label.len()
///
/// "ED25519 256" (11 chars) → 2 dashes → "+--[ED25519 256]--+"
/// "PERIPHORE"   (9 chars)  → 4 dashes → "+--[PERIPHORE]----+"
fn build_border(label: &str) -> String {
    let dash_count = 13 - label.len();
    format!("+--[{}]{:->width$}+", label, "", width = dash_count)
}

/// Extract 6 sequential 11-bit indices from a 32-byte big-endian fingerprint.
///
/// Used to select BIP39 words. The i-th window starts at bit offset i*11.
/// Each window spans at most 3 bytes to guarantee alignment.
///
/// Python cross-validation confirmed this matches big-integer extraction (RESEARCH.md §5).
fn word_indices(fingerprint: &[u8; 32]) -> [usize; 6] {
    let mut indices = [0usize; 6];
    for i in 0..6 {
        let bit_offset = i * 11;
        let byte_offset = bit_offset / 8;
        let bit_shift = bit_offset % 8;
        #[allow(clippy::cast_possible_truncation)]
        let window = ((fingerprint[byte_offset] as u32) << 16
            | (fingerprint[byte_offset + 1] as u32) << 8
            | (fingerprint[byte_offset + 2] as u32))
            >> (13 - bit_shift);
        indices[i] = (window & 0x7FF) as usize;
    }
    indices
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
