//! Integration tests for periphore-config layered loading.
//! Tests verify: defaults load, TOML overrides defaults, env overrides TOML.
//! Compile-time CFG-01 invariant: Config has no Serialize impl.
//!
//! NOTE: Tests that modify environment variables use a shared mutex to prevent
//! concurrent access. Env vars are process-global state, so tests that set
//! PERIPHORE_* vars must not run in parallel with tests that read config.

use std::io::Write;
use std::sync::Mutex;

use periphore_config::load;

/// Mutex to serialize tests that depend on environment variable state.
/// Figment reads PERIPHORE_* env vars on every `load()` call, so concurrent
/// tests that set/clear these vars would interfere with each other.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Clear any PERIPHORE_ env vars that could leak between tests.
fn clear_periphore_env() {
    // Safety: these are test-only env var mutations; the ENV_MUTEX ensures
    // no concurrent test is reading config while we mutate env state.
    unsafe { std::env::remove_var("PERIPHORE_LOGGING_LEVEL") };
}

#[test]
fn defaults_load_without_file() {
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    // load() with no file path should succeed using compiled-in defaults.
    let config = load(None).expect("default config should load without error");
    // Logging level default is "info"
    assert_eq!(config.logging.level, "info");
}

#[test]
fn toml_file_overrides_defaults() {
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    // Write a temp TOML file that overrides the logging level.
    let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
    writeln!(tmp, "[logging]").unwrap();
    writeln!(tmp, r#"level = "debug""#).unwrap();

    let config = load(Some(tmp.path())).expect("should load with TOML file");
    assert_eq!(config.logging.level, "debug");
}

#[test]
fn env_overrides_toml_file() {
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    // Environment variable takes precedence over TOML file (env is higher priority).
    // Using PERIPHORE_LOGGING_LEVEL (Env::prefixed("PERIPHORE_").split("_") maps
    // PERIPHORE_LOGGING_LEVEL -> logging.level via nested key splitting).
    let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
    writeln!(tmp, "[logging]").unwrap();
    writeln!(tmp, r#"level = "warn""#).unwrap();

    // Set env var -- PERIPHORE_LOGGING_LEVEL maps to logging.level via split("_")
    unsafe { std::env::set_var("PERIPHORE_LOGGING_LEVEL", "trace") };
    let config = load(Some(tmp.path())).expect("should load with env override");
    unsafe { std::env::remove_var("PERIPHORE_LOGGING_LEVEL") };

    // Env (trace) must win over TOML (warn)
    assert_eq!(config.logging.level, "trace");
}

#[test]
fn missing_toml_file_is_ignored() {
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    // If the config file path doesn't exist, load() should succeed with defaults
    // (not return an error). This is important for first-run experience.
    let nonexistent = std::path::Path::new("/tmp/periphore-nonexistent-config-xyz.toml");
    let config = load(Some(nonexistent)).expect("missing config file should not error");
    assert_eq!(config.logging.level, "info"); // default
}

#[test]
fn peer_config_vec_defaults_to_empty() {
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    let config = load(None).expect("default config");
    assert!(config.peers.is_empty(), "peers should default to empty vec");
}

#[test]
fn identity_show_identicon_defaults_to_true() {
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    let config = load(None).expect("default config should load");
    assert!(
        config.identity.show_identicon,
        "identity.show_identicon must default to true"
    );
}

#[test]
fn identity_show_identicon_can_be_disabled_via_toml() {
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
    writeln!(tmp, "[identity]").unwrap();
    writeln!(tmp, "show_identicon = false").unwrap();

    let config = load(Some(tmp.path())).expect("should load with identity config");
    assert!(
        !config.identity.show_identicon,
        "identity.show_identicon must be false when set in TOML"
    );
}
