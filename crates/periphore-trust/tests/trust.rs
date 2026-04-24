//! Integration tests for periphore-trust.
//! All tests live here because [lib] test = false in Cargo.toml.
//!
//! SEC-05 tests: trust cache persistence (add, reload, corrupt, separate file)
//! SEC-06 tests: fingerprint conflict detection (match, mismatch, case insensitivity)

use periphore_trust::{TrustError, TrustStore, check_peer_fingerprint};

// ---------------------------------------------------------------------------
// SEC-05: Trust cache persistence
// ---------------------------------------------------------------------------

#[test]
fn test_add_trusted_persists_across_reload() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache_path = dir.path().join("trusted.toml");
    let fp = "a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9";

    let mut store = TrustStore::load(&cache_path).expect("initial load");
    store.add_trusted(fp, Some("test-peer"), &cache_path).expect("add_trusted");

    drop(store);
    let store2 = TrustStore::load(&cache_path).expect("reload");
    assert!(store2.is_trusted(fp), "fingerprint must persist across reload");
}

#[test]
fn test_cache_separate_from_config() {
    let path = periphore_trust::default_trust_path();
    assert!(path.is_some(), "default_trust_path must return Some on dev machine");
    let path = path.unwrap();
    assert!(
        path.to_str().unwrap().ends_with("trusted.toml"),
        "trust cache path must end with trusted.toml, got: {}",
        path.display()
    );
    // Must be in data_dir, not config_dir — verifies separation from main config.
    assert!(
        !path.to_str().unwrap().contains(".config"),
        "trust cache must NOT be in .config dir (separate from main config), got: {}",
        path.display()
    );
}

#[test]
fn test_corrupt_cache_returns_error() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache_path = dir.path().join("trusted.toml");

    std::fs::write(&cache_path, "this is {{ not valid toml {{{{").expect("write corrupt file");
    let result = TrustStore::load(&cache_path);
    match result {
        Err(TrustError::CorruptCacheFile(_)) => { /* expected */ }
        other => panic!("expected CorruptCacheFile, got: {other:?}"),
    }
}

#[test]
fn test_load_missing_file_returns_empty() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache_path = dir.path().join("nonexistent.toml");

    let store = TrustStore::load(&cache_path).expect("load of missing file must succeed");
    assert!(
        !store.is_trusted("anything"),
        "empty trust store must not trust any fingerprint"
    );
}

#[test]
fn test_add_trusted_idempotent() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache_path = dir.path().join("trusted.toml");
    let fp = "a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9";

    let mut store = TrustStore::load(&cache_path).expect("load");
    store.add_trusted(fp, None, &cache_path).expect("first add");
    store.add_trusted(fp, None, &cache_path).expect("second add must also succeed (idempotent)");
    assert!(store.is_trusted(fp));
}

#[test]
fn test_remove_trusted() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache_path = dir.path().join("trusted.toml");
    let fp = "a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9";

    let mut store = TrustStore::load(&cache_path).expect("load");
    store.add_trusted(fp, None, &cache_path).expect("add");
    store.remove_trusted(fp, &cache_path).expect("remove");
    assert!(!store.is_trusted(fp), "fingerprint must not be trusted after removal");
}

#[test]
fn test_remove_nonexistent_returns_not_found() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache_path = dir.path().join("trusted.toml");

    let mut store = TrustStore::load(&cache_path).expect("load");
    let result = store.remove_trusted("0000000000000000000000000000000000000000000000000000000000000000", &cache_path);
    match result {
        Err(TrustError::NotFound(_)) => { /* expected */ }
        other => panic!("expected NotFound, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// SEC-06: Fingerprint conflict detection
// ---------------------------------------------------------------------------

#[test]
fn test_fingerprint_conflict_detected() {
    let configured = "a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9";
    let actual     = "b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1";

    let result = check_peer_fingerprint(configured, actual, "test-peer");
    match result {
        Err(TrustError::FingerprintConflict { expected, actual: got, peer_label }) => {
            assert_eq!(expected, configured);
            assert_eq!(got, actual);
            assert_eq!(peer_label, "test-peer");
        }
        other => panic!("expected FingerprintConflict, got: {other:?}"),
    }
}

#[test]
fn test_matching_fingerprint_passes() {
    let fp = "a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9";
    check_peer_fingerprint(fp, fp, "test-peer").expect("matching fingerprints must pass");
}

#[test]
fn test_fingerprint_case_insensitive() {
    let lower = "a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9";
    let upper = "A3F9A3F9A3F9A3F9A3F9A3F9A3F9A3F9A3F9A3F9A3F9A3F9A3F9A3F9A3F9A3F9";
    check_peer_fingerprint(lower, upper, "test-peer")
        .expect("case-insensitive comparison must pass");
    check_peer_fingerprint(upper, lower, "test-peer")
        .expect("case-insensitive comparison must pass (reversed)");
}
