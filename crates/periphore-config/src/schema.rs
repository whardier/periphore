use serde::Deserialize;

// CRITICAL: Config intentionally does NOT derive Serialize.
// This enforces CFG-01 at compile time: no code path can serialize Config to disk.
// Fingerprint cache (SEC-05) uses a separate path managed by trust acceptance flow,
// NOT this config crate. See D-24.
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub daemon:    DaemonConfig,
    #[serde(default)]
    pub logging:   LoggingConfig,
    #[serde(default, rename = "peer")]
    pub peers:     Vec<PeerConfig>,
    #[serde(default)]
    pub topology:  TopologyConfig,
    #[serde(default)]
    pub capture:   CaptureConfig,
    #[serde(default)]
    pub identity:  IdentityConfig,
}

/// Daemon process configuration.
#[derive(Debug, Deserialize)]
pub struct DaemonConfig {
    /// Override for the IPC socket path. If None, platform default is used (via periphore-ipc).
    pub socket_path: Option<std::path::PathBuf>,
    /// TCP port the daemon listens on for peer connections (Phase 6).
    pub port: Option<u16>,
    /// Whether the daemon listens for incoming peer TCP connections.
    /// Default: true (P2P symmetric model, D-07).
    /// Set to false for CI/testing setups that should not accept incoming peers.
    #[serde(default = "default_listen")]
    pub listen: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: None,
            port:        None,
            listen:      true,
        }
    }
}

fn default_listen() -> bool {
    true
}

/// Logging configuration.
#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    /// Log level: error, warn, info, debug, trace.
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_owned(),
        }
    }
}

/// Per-peer configuration. Repeated as [[peer]] in TOML.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PeerConfig {
    /// Expected peer fingerprint (hex string). Optional -- if set, connection from non-matching
    /// fingerprint is refused (hard config enforcement, Phase 3 SEC-06).
    pub fingerprint: Option<String>,
    /// Human-readable label for this peer. Local-only convenience -- NOT sent over
    /// the wire, does NOT participate in identity verification. Used in log
    /// messages and error reports. If absent, logs use the fingerprint hex
    /// or host address.
    pub name: Option<String>,
    /// Manual host for connecting to this peer (Phase 6 NET-03).
    pub host: Option<String>,
    /// TCP port override for this peer.
    pub port: Option<u16>,
}

/// Per-monitor configuration entry. Repeated as [[topology.monitor]] in TOML.
/// Phase 3 defines the schema; Phase 8 adds processing logic.
#[derive(Debug, Deserialize, Default)]
pub struct MonitorConfig {
    /// OS-level monitor identifier (xrandr output name, CoreGraphics display ID, etc.).
    /// Free-form string -- Phase 8 implements matching against OS-provided identifiers.
    /// Local per-node; no cross-node uniqueness requirement.
    pub id: Option<String>,
    /// Human-readable label (optional override for display in logs/CLI).
    pub name: Option<String>,
    /// Monitor width in pixels.
    pub width: Option<u32>,
    /// Monitor height in pixels.
    pub height: Option<u32>,
}

/// Monitor topology configuration (Phase 3: monitor entries; Phase 8: edge config).
#[derive(Debug, Deserialize, Default)]
pub struct TopologyConfig {
    /// Preferred monitor layout entries. TOML: [[topology.monitor]].
    /// The serde rename maps the TOML key "monitor" to the Rust field "monitors",
    /// so [[topology.monitor]] entries deserialize into this Vec.
    #[serde(default, rename = "monitor")]
    pub monitors: Vec<MonitorConfig>,
}

/// Input capture configuration (Phase 9 fills this in).
#[derive(Debug, Deserialize, Default)]
pub struct CaptureConfig {
    // Captive window vs seamless mode, device path overrides -- populated in Phase 9.
}

/// Identity display configuration (SEC-04).
#[derive(Debug, Deserialize)]
pub struct IdentityConfig {
    /// Show identicon on startup and in IPC GetIdenticon responses.
    /// Set to `false` for headless or automated setups (SEC-04).
    ///
    /// Note: this field contains an underscore. Figment's `Env::prefixed("PERIPHORE_").split("_")`
    /// would map `PERIPHORE_IDENTITY_SHOW_IDENTICON` to `identity.show.identicon` (wrong — 3 levels).
    /// Configure via the TOML file only: `[identity]\nshow_identicon = false`.
    pub show_identicon: bool,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self { show_identicon: true }
    }
}
