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
    // SAFETY: ENV_MUTEX serializes all PERIPHORE_* env var mutations in this
    // test binary. This assumes no background thread (e.g., from figment,
    // tempfile, or tokio) reads PERIPHORE_* vars concurrently. If that
    // assumption breaks, move to process-isolated tests (separate test binary
    // per env-sensitive test).
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

    // SAFETY: ENV_MUTEX held; see clear_periphore_env() for full safety rationale.
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

// ---------------------------------------------------------------------------
// NET-04 / D-07: DaemonConfig.listen field (Phase 6)
// ---------------------------------------------------------------------------

#[test]
fn daemon_listen_defaults_to_true() {
    // D-07: daemon.listen must default to true (P2P symmetric model).
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    let config = load(None).expect("default config should load");
    assert!(
        config.daemon.listen,
        "daemon.listen must default to true"
    );
}

#[test]
fn daemon_listen_can_be_set_false_via_toml() {
    // D-07: daemon.listen = false disables TCP listener for CI/testing setups.
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
    writeln!(tmp, "[daemon]").unwrap();
    writeln!(tmp, "listen = false").unwrap();

    let config = load(Some(tmp.path())).expect("should load with daemon.listen = false");
    assert!(
        !config.daemon.listen,
        "daemon.listen must be false when set to false in TOML"
    );
}

#[test]
fn daemon_listen_true_when_absent_from_toml() {
    // D-07: TOML without listen field must produce listen = true via serde default.
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
    writeln!(tmp, "[daemon]").unwrap();
    // Intentionally no listen field — serde default must kick in
    writeln!(tmp, "# port = 7888").unwrap();

    let config = load(Some(tmp.path())).expect("should load with partial daemon section");
    assert!(
        config.daemon.listen,
        "daemon.listen must be true when absent from TOML [daemon] section"
    );
}

// ---------------------------------------------------------------------------
// CFG-02: PeerConfig.name field (Phase 3)
// ---------------------------------------------------------------------------

#[test]
fn test_peer_name_field() {
    // CFG-02: PeerConfig.name parses from TOML [[peer]] block.
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
    writeln!(tmp, "[[peer]]").unwrap();
    writeln!(tmp, r#"name = "work-mac""#).unwrap();
    writeln!(tmp, r#"host = "192.168.1.100""#).unwrap();
    writeln!(tmp, "port = 24800").unwrap();

    let config = load(Some(tmp.path())).expect("should load with peer name");
    assert_eq!(config.peers.len(), 1, "must have 1 peer entry");
    assert_eq!(
        config.peers[0].name,
        Some("work-mac".to_owned()),
        "peer name must parse from TOML"
    );
}

#[test]
fn test_peer_name_defaults_to_none() {
    // CFG-02: PeerConfig without a name field defaults to None.
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
    writeln!(tmp, "[[peer]]").unwrap();
    writeln!(tmp, r#"host = "10.0.0.1""#).unwrap();

    let config = load(Some(tmp.path())).expect("should load peer without name");
    assert_eq!(config.peers.len(), 1);
    assert_eq!(
        config.peers[0].name, None,
        "peer name must default to None when absent"
    );
}

// ---------------------------------------------------------------------------
// CFG-03: TopologyConfig.monitors (Phase 3)
// ---------------------------------------------------------------------------

#[test]
fn test_topology_monitor_config() {
    // CFG-03: [[topology.monitor]] entries parse into TopologyConfig.monitors vec.
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
    writeln!(tmp, "[[topology.monitor]]").unwrap();
    writeln!(tmp, r#"id = "DP-1""#).unwrap();
    writeln!(tmp, r#"name = "primary""#).unwrap();
    writeln!(tmp, "width = 2560").unwrap();
    writeln!(tmp, "height = 1440").unwrap();
    writeln!(tmp, "").unwrap();
    writeln!(tmp, "[[topology.monitor]]").unwrap();
    writeln!(tmp, r#"id = "HDMI-1""#).unwrap();
    writeln!(tmp, r#"name = "secondary""#).unwrap();
    writeln!(tmp, "width = 1920").unwrap();
    writeln!(tmp, "height = 1080").unwrap();

    let config = load(Some(tmp.path())).expect("should load with monitor config");
    assert_eq!(
        config.topology.monitors.len(), 2,
        "must have 2 monitor entries"
    );
    assert_eq!(config.topology.monitors[0].id, Some("DP-1".to_owned()));
    assert_eq!(config.topology.monitors[0].name, Some("primary".to_owned()));
    assert_eq!(config.topology.monitors[0].width, Some(2560));
    assert_eq!(config.topology.monitors[0].height, Some(1440));
    assert_eq!(config.topology.monitors[1].id, Some("HDMI-1".to_owned()));
    assert_eq!(config.topology.monitors[1].width, Some(1920));
}

#[test]
fn test_topology_monitors_default_empty() {
    // CFG-03: No [[topology.monitor]] in TOML means monitors defaults to empty vec.
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    let config = load(None).expect("default config");
    assert!(
        config.topology.monitors.is_empty(),
        "monitors must default to empty vec"
    );
}
