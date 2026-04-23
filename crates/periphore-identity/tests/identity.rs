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
// SEC-02: Identicon (Drunken Bishop) — plan 02-02
// ---------------------------------------------------------------------------

#[test]
fn test_identicon_determinism() {
    // Same fingerprint always produces identical identicon string.
    let dir = tempfile::tempdir().expect("temp dir");
    let key_path = dir.path().join("key");
    const TEST_SEED: [u8; 32] = [0u8; 32];
    fs::write(&key_path, TEST_SEED).expect("write test seed");

    let store = IdentityStore::load_or_create(&key_path).expect("load");
    let first = store.identicon();
    let second = store.identicon();
    assert_eq!(first, second, "identicon must be deterministic");
    assert!(!first.is_empty(), "identicon must not be empty");
}

#[test]
fn test_identicon_borders() {
    // Header must be "+--[ED25519 256]--+" and footer "+--[PERIPHORE]----+".
    let dir = tempfile::tempdir().expect("temp dir");
    let key_path = dir.path().join("key");
    const TEST_SEED: [u8; 32] = [0u8; 32];
    fs::write(&key_path, TEST_SEED).expect("write test seed");

    let store = IdentityStore::load_or_create(&key_path).expect("load");
    let identicon = store.identicon();
    let lines: Vec<&str> = identicon.lines().collect();

    assert_eq!(
        lines[0], "+--[ED25519 256]--+",
        "header must match exactly"
    );
    assert_eq!(
        lines[10], "+--[PERIPHORE]----+",
        "footer must match exactly (line index 10)"
    );
    // Each grid row must be 19 chars wide (| + 17 + |)
    for (i, line) in lines[1..10].iter().enumerate() {
        assert_eq!(
            line.len(), 19,
            "grid row {} must be 19 chars wide, got {}", i + 1, line.len()
        );
        assert!(
            line.starts_with('|') && line.ends_with('|'),
            "grid row {} must start and end with |", i + 1
        );
    }
}

#[test]
fn test_identicon_line_count() {
    // Output has exactly 11 lines (header + 9 grid rows + footer).
    let dir = tempfile::tempdir().expect("temp dir");
    let key_path = dir.path().join("key");
    const TEST_SEED: [u8; 32] = [0u8; 32];
    fs::write(&key_path, TEST_SEED).expect("write test seed");

    let store = IdentityStore::load_or_create(&key_path).expect("load");
    let identicon = store.identicon();
    let line_count = identicon.lines().count();
    assert_eq!(line_count, 11, "identicon must have 11 lines, got {line_count}");
}

// ---------------------------------------------------------------------------
// SEC-03: Word phrase (BIP39) — plan 02-02
// ---------------------------------------------------------------------------

#[test]
fn test_word_phrase_determinism() {
    // Same fingerprint always produces the same 6-word phrase.
    let dir = tempfile::tempdir().expect("temp dir");
    let key_path = dir.path().join("key");
    const TEST_SEED: [u8; 32] = [0u8; 32];
    fs::write(&key_path, TEST_SEED).expect("write test seed");

    let store = IdentityStore::load_or_create(&key_path).expect("load");
    let first = store.word_phrase();
    let second = store.word_phrase();
    assert_eq!(first, second, "word_phrase must be deterministic");
    assert_eq!(first.len(), 6, "word_phrase must have 6 words");
}

#[test]
fn test_word_phrase_validity() {
    // All 6 words must be lowercase and present in BIP39_WORDS.
    // phrase (joined) must be space-delimited with exactly 5 spaces.
    let dir = tempfile::tempdir().expect("temp dir");
    let key_path = dir.path().join("key");
    const TEST_SEED: [u8; 32] = [0u8; 32];
    fs::write(&key_path, TEST_SEED).expect("write test seed");

    let store = IdentityStore::load_or_create(&key_path).expect("load");
    let words = store.word_phrase();

    assert_eq!(words.len(), 6, "must have exactly 6 words");
    // Validate via known BIP39 properties: all lowercase, no punctuation.
    for word in &words {
        assert!(
            word.chars().all(|c| c.is_ascii_lowercase()),
            "word '{word}' must be all lowercase ASCII"
        );
        assert!(!word.is_empty(), "word must not be empty");
    }
    // Joined phrase must be space-delimited.
    let phrase = words.join(" ");
    let space_count = phrase.chars().filter(|&c| c == ' ').count();
    assert_eq!(space_count, 5, "phrase must have exactly 5 spaces between 6 words");
}
