//! Integration and unit tests for periphore-identity.
//! All tests live here because [lib] test = false in Cargo.toml (Phase 1 D-07).
//!
//! SEC-01 tests: test_first_run_creates_key_file, test_load_after_create_is_identical,
//!               test_corrupt_key_file_error, test_fingerprint_determinism
//! SEC-02 tests: test_identicon_determinism, test_identicon_borders, test_identicon_line_count
//!               (implemented in plan 02-02; stubs here)
//! SEC-03 tests: test_word_phrase_determinism, test_word_phrase_validity
//!               (implemented in plan 02-02; stubs here)

use std::fs;

use periphore_identity::{IdentityError, IdentityStore};

// ---------------------------------------------------------------------------
// SEC-01: Keypair persistence and fingerprint derivation
// ---------------------------------------------------------------------------

#[test]
fn test_first_run_creates_key_file() {
    // load_or_create on a non-existent path must create the key file.
    let dir = tempfile::tempdir().expect("temp dir");
    let key_path = dir.path().join("key");

    assert!(!key_path.exists(), "key must not exist before first run");
    let _store = IdentityStore::load_or_create(&key_path)
        .expect("load_or_create must succeed on first run");
    assert!(key_path.exists(), "key file must exist after load_or_create");

    // Key file must be exactly 32 bytes (raw Ed25519 seed — D-01).
    let bytes = fs::read(&key_path).expect("read key file");
    assert_eq!(bytes.len(), 32, "key file must be exactly 32 bytes, got {}", bytes.len());
}

#[test]
fn test_load_after_create_is_identical() {
    // Two sequential load_or_create calls on the same path must produce
    // the same fingerprint (load, not re-generate).
    let dir = tempfile::tempdir().expect("temp dir");
    let key_path = dir.path().join("key");

    let first = IdentityStore::load_or_create(&key_path)
        .expect("first load_or_create");
    let second = IdentityStore::load_or_create(&key_path)
        .expect("second load_or_create");

    assert_eq!(
        first.fingerprint_hex(),
        second.fingerprint_hex(),
        "fingerprint must be identical on reload"
    );
}

#[test]
fn test_corrupt_key_file_error() {
    // A key file with wrong byte count must return IdentityError::CorruptKeyFile.
    let dir = tempfile::tempdir().expect("temp dir");
    let key_path = dir.path().join("key");

    // Write 16 bytes (wrong length — correct is 32).
    fs::write(&key_path, vec![0u8; 16]).expect("write corrupt file");

    let result = IdentityStore::load_or_create(&key_path);
    match result {
        Err(IdentityError::CorruptKeyFile(16)) => { /* expected */ }
        other => panic!("expected CorruptKeyFile(16), got: {other:?}"),
    }
}

#[test]
fn test_fingerprint_determinism() {
    // Same 32-byte seed always produces the same 64-char lowercase hex fingerprint.
    // Uses the all-zeros test seed — never use in production.
    let dir = tempfile::tempdir().expect("temp dir");
    let key_path = dir.path().join("key");

    // Write a known seed directly to bypass the CSPRNG.
    const TEST_SEED: [u8; 32] = [0u8; 32];
    fs::write(&key_path, TEST_SEED).expect("write test seed");

    let store = IdentityStore::load_or_create(&key_path)
        .expect("load from known seed");
    let hex = store.fingerprint_hex();

    // Must be 64 lowercase hex characters.
    assert_eq!(hex.len(), 64, "fingerprint_hex must be 64 chars, got {}", hex.len());
    assert!(
        hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
        "fingerprint_hex must be lowercase hex, got: {hex}"
    );

    // Must be deterministic: second call from same seed produces identical result.
    let store2 = IdentityStore::load_or_create(&key_path)
        .expect("second load from same seed");
    assert_eq!(
        hex,
        store2.fingerprint_hex(),
        "fingerprint_hex must be identical for the same seed"
    );
}

// ---------------------------------------------------------------------------
// SEC-02: Identicon (Drunken Bishop) — implemented in plan 02-02
// ---------------------------------------------------------------------------

#[test]
#[ignore = "implemented in plan 02-02"]
fn test_identicon_determinism() {}

#[test]
#[ignore = "implemented in plan 02-02"]
fn test_identicon_borders() {}

#[test]
#[ignore = "implemented in plan 02-02"]
fn test_identicon_line_count() {}

// ---------------------------------------------------------------------------
// SEC-03: Word phrase (BIP39) — implemented in plan 02-02
// ---------------------------------------------------------------------------

#[test]
#[ignore = "implemented in plan 02-02"]
fn test_word_phrase_determinism() {}

#[test]
#[ignore = "implemented in plan 02-02"]
fn test_word_phrase_validity() {}
