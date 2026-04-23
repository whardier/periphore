//! periphore-config: layered configuration loading for Periphore.
//!
//! Loading order (last wins, per Figment semantics):
//! 1. Compiled-in defaults (via `#[serde(default)]` on all Config fields)
//! 2. TOML config file (`Toml::file` -- optional, missing file is ignored)
//! 3. Environment variables with `PERIPHORE_` prefix (`Env::prefixed`)
//! 4. CLI argument overrides (merged by the binary caller, not this crate -- D-22)
//!
//! CFG-01 invariant: Config has no Serialize impl. No write path exists in this crate.
//! Defaults are provided via serde's `#[serde(default)]` attribute and `Default` impls
//! rather than Figment's `Serialized::defaults` (which would require Serialize).

mod schema;

pub use schema::{CaptureConfig, Config, DaemonConfig, LoggingConfig, PeerConfig, TopologyConfig};

use figment::{
    providers::{Env, Format, Toml},
    Figment,
};

/// Errors from config loading.
#[derive(Debug)]
pub struct ConfigError(figment::Error);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "config error: {}", self.0)
    }
}

impl std::error::Error for ConfigError {}

impl From<figment::Error> for ConfigError {
    fn from(e: figment::Error) -> Self {
        Self(e)
    }
}

/// Load configuration with Figment layered precedence:
/// defaults < TOML file < env vars (< CLI overrides merged by caller).
///
/// If `config_path` is `None` or the file does not exist, only defaults and env vars are used.
/// A missing config file is not an error -- it is a valid first-run state.
///
/// Defaults are provided by serde's `#[serde(default)]` annotations on Config fields
/// combined with `Default` trait impls. This avoids requiring `Serialize` on Config,
/// enforcing CFG-01 (no config auto-write) at the type system level.
///
/// # Environment variable mapping constraint
///
/// `Env::prefixed("PERIPHORE_").split("_")` splits on every underscore after stripping
/// the prefix to produce a nested key path. This means:
///
/// - `PERIPHORE_LOGGING_LEVEL` maps to `logging.level` (2 levels — correct)
/// - `PERIPHORE_DAEMON_SOCKET_PATH` would map to `daemon.socket.path` (3 levels — **wrong**)
///
/// **IMPORTANT:** Field names within config structs MUST NOT contain underscores, or the
/// env var mapping will silently produce a key path with too many levels and fall back to
/// the default value without any error. `DaemonConfig::socket_path` is exempt because it
/// is not expected to be configurable via env vars (use the TOML file or CLI flag instead).
/// Before Phase 5 wires up full env override support, verify that no underscore-bearing
/// field name requires env var override.
pub fn load(config_path: Option<&std::path::Path>) -> Result<Config, ConfigError> {
    // Start with an empty Figment. Defaults come from #[serde(default)] on Config
    // fields and their Default impls -- serde fills in missing keys automatically.
    let mut figment = Figment::new();

    // Middle priority: TOML file (if path provided).
    // Figment silently ignores non-existent files for Toml::file() -- this is correct
    // for first-run experience where no config file has been authored yet.
    if let Some(path) = config_path {
        figment = figment.merge(Toml::file(path));
    }

    // Higher priority: environment variables prefixed with PERIPHORE_.
    // split("_") maps PERIPHORE_LOGGING_LEVEL -> logging.level (nested key path).
    figment = figment.merge(Env::prefixed("PERIPHORE_").split("_"));

    // CLI arg overrides are NOT merged here. The binary caller merges them last
    // (highest priority) using a Serialized or custom provider. (D-22, D-25)

    figment.extract().map_err(ConfigError::from)
}
