# Phase 2: Identity & Cryptography — Pattern Map

**Mapped:** 2026-04-22
**Files analyzed:** 9
**Analogs found:** 9 / 9

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/periphore-identity/src/lib.rs` | service/library | file-I/O + transform | `crates/periphore-ipc/src/server.rs` (file perms + error pattern) | role-partial |
| `crates/periphore-identity/src/bip39.rs` | utility | transform | `crates/periphore-protocol/src/types.rs` (static data module) | role-match |
| `crates/periphore-identity/Cargo.toml` | config | — | `crates/periphore-ipc/Cargo.toml` (thiserror workspace dep pattern) | exact |
| `crates/periphore-identity/tests/identity.rs` | test | — | `crates/periphore-protocol/tests/roundtrip.rs` (no file exists yet — new pattern) | no-analog |
| `crates/periphore-protocol/src/ipc.rs` | model | request-response | `crates/periphore-protocol/src/ipc.rs` itself (extend existing enum) | exact |
| `crates/periphore-config/src/schema.rs` | model/config | — | `crates/periphore-config/src/schema.rs` itself (extend existing structs) | exact |
| `crates/periphored/src/main.rs` | controller | request-response | `crates/periphored/src/main.rs` itself (existing select! dispatch pattern) | exact |
| `crates/periphored/Cargo.toml` | config | — | `crates/periphored/Cargo.toml` itself (add workspace dep) | exact |
| `Cargo.toml` (workspace) | config | — | `Cargo.toml` (workspace) itself (add to `[workspace.dependencies]`) | exact |

---

## Pattern Assignments

### `crates/periphore-identity/src/lib.rs` (service/library, file-I/O + transform)

**Primary analog:** `crates/periphore-ipc/src/server.rs` — file permissions pattern
**Secondary analog:** `crates/periphore-config/src/lib.rs` — `thiserror`-free but manual error type, showing how library crates expose errors

**Imports pattern** — copy this structure for lib.rs (from `crates/periphore-ipc/src/server.rs` lines 1-9 and `crates/periphore-config/src/lib.rs` lines 17-22):

```rust
use std::fs;
use std::path::Path;
// from server.rs — PermissionsExt for 0600 file mode
// #[cfg(unix)] use std::os::unix::fs::PermissionsExt;

use periphore_protocol::{IpcRequest, IpcResponse};  // (server.rs pattern — cross-crate import)
```

For identity lib.rs, the imports block will be:
```rust
use std::io;
use std::path::{Path, PathBuf};

use ed25519_dalek::SigningKey;
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use thiserror::Error;
use directories::ProjectDirs;
```

**Unix file permissions pattern** (from `crates/periphore-ipc/src/server.rs` lines 27-35 and 41-46):

```rust
// create_dir_all + 0700 for directory (server.rs lines 27-35)
if let Some(parent) = socket_path.parent() {
    fs::create_dir_all(parent)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }
}

// 0600 for the file itself (server.rs lines 41-46)
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(socket_path, fs::Permissions::from_mode(0o600))?;
}
```

For identity, use `OpenOptionsExt::mode(0o600)` at creation (RESEARCH.md Pitfall 6 — better than post-write set_permissions):
```rust
use std::os::unix::fs::OpenOptionsExt;

let mut file = std::fs::OpenOptions::new()
    .write(true)
    .create_new(true)
    .mode(0o600)
    .open(path)?;
file.write_all(&seed)?;
```

**Error type pattern** — copy from `crates/periphore-config/src/lib.rs` lines 23-38, adapted to use `thiserror`:

The config crate has a manual error newtype (no `thiserror` there). For identity, use `thiserror` as established in the workspace. The pattern for a `thiserror`-derived error in this workspace is:

```rust
// Pattern: thiserror derive with #[from] for std::io::Error
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("key file is corrupt (expected 32 bytes, got {0})")]
    CorruptKeyFile(usize),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("no valid data directory found for this user")]
    NoDataDir,
}
```

**`anyhow` boundary pattern** — `periphored/src/main.rs` lines 45-46 show the daemon → library error conversion at the `anyhow` boundary:

```rust
// From periphored/src/main.rs lines 45-46:
let config = periphore_config::load(args.config.as_deref())
    .map_err(|e| anyhow::anyhow!("failed to load config: {e}"))?;
```

Apply the same conversion for identity:
```rust
let identity = periphore_identity::IdentityStore::load_or_create(&key_path)
    .map_err(|e| anyhow::anyhow!("identity error: {e}"))?;
```

**`tracing::info!` structured log pattern** — from `periphored/src/main.rs` lines 48-51:

```rust
// Named field syntax (= %value for Display):
tracing::info!(
    log_level = %config.logging.level,
    "periphored starting"
);

// Apply same pattern for identity first-run log:
tracing::info!(
    fingerprint = %identity.fingerprint_hex(),
    "identity loaded"
);
// On first-run creation (inside IdentityStore::load_or_create):
tracing::info!("Generated new identity: {}", fingerprint_hex);
```

---

### `crates/periphore-identity/src/bip39.rs` (utility, transform)

**Analog:** `crates/periphore-protocol/src/types.rs` — static data and pure type definitions module.

This is a pure data module. Pattern: declare a `pub(crate)` static with a compile-time length assertion. No analog file exists in the current codebase for an inline wordlist, but the `types.rs` module structure is the closest match for a self-contained data module.

**Module pattern** — from `crates/periphore-protocol/src/lib.rs` lines 6-8 (module declaration):

```rust
// In lib.rs — declare the submodule:
mod bip39;
// Make word list accessible internally:
use crate::bip39::BIP39_WORDS;
```

**Compile-time assertion pattern** — add after the static declaration:

```rust
// In bip39.rs:
pub(crate) static BIP39_WORDS: &[&str; 2048] = &[
    "abandon", "ability", "able", /* ... all 2048 words ... */
];

// Compile-time length guard (RESEARCH.md Pitfall 8):
const _: () = assert!(BIP39_WORDS.len() == 2048);
```

---

### `crates/periphore-identity/Cargo.toml` (config)

**Analog:** `crates/periphore-identity/Cargo.toml` itself (lines 1-20) + workspace `Cargo.toml` lines 41-43 for the workspace dep reference pattern.

**Workspace dep reference pattern** — current identity Cargo.toml lines 17-20 show the established pattern:

```toml
# Existing pattern (periphore-identity/Cargo.toml lines 17-20):
[dependencies]
ed25519-dalek = { workspace = true }
sha2          = { workspace = true }
serde         = { workspace = true }
```

**Add these lines** to `[dependencies]`:

```toml
rand_core     = { workspace = true }
thiserror     = { workspace = true }
directories   = { workspace = true }
```

**Feature override at crate level** (RESEARCH.md — add `rand_core` feature to ed25519-dalek only in this crate):

```toml
ed25519-dalek = { workspace = true, features = ["rand_core"] }
```

**`[lib]` section** — copy from existing identity Cargo.toml lines 11-13 (already correct, do not change):

```toml
[lib]
doctest = false
test    = false
```

---

### `crates/periphore-identity/tests/identity.rs` (test, no existing analog)

**No analog exists** — this is the first integration test file in this crate. No `tests/` subdirectory exists in any crate yet (`Glob("**/tests/*.rs")` returned empty).

**Test file structure to use** — standard Rust integration test pattern for a library crate with `test = false` in `[lib]`:

```rust
// Integration tests live in tests/ subdir because [lib] test = false (D-07 from Phase 1)
// Each test function is a standalone binary linked against the crate's public API.

use std::path::PathBuf;
use tempfile::TempDir;  // or use std::env::temp_dir() if no tempfile dep available

use periphore_identity::{IdentityError, IdentityStore};

#[test]
fn test_first_run_creates_key_file() {
    // ...
}

#[test]
fn test_load_after_create_is_identical() {
    // ...
}

#[test]
fn test_corrupt_key_file_error() {
    // ...
}

#[test]
fn test_fingerprint_determinism() {
    // Known seed → known fingerprint hex (golden value)
    const TEST_SEED: [u8; 32] = [0u8; 32];
    // ...
}
```

**Note on test dependencies:** The test file may need `tempfile` crate for temp directory management, or use `std::env::temp_dir()` with a unique name. Check whether `tempfile` is needed and if so add it to `periphore-identity/Cargo.toml` under `[dev-dependencies]` (not `[dependencies]`).

---

### `crates/periphore-protocol/src/ipc.rs` (model, request-response — extend existing)

**Analog:** `crates/periphore-protocol/src/ipc.rs` lines 41-55 — the existing `IpcResponse` enum.

**Current `IpcResponse` enum** (lines 41-55 — copy this and add new variants):

```rust
// Current (periphore-protocol/src/ipc.rs lines 41-55):
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum IpcResponse {
    Status {
        running:     bool,
        fingerprint: Option<String>,
    },
    Peers {
        peers: Vec<String>,
    },
    Ok,
    Error {
        message: String,
    },
}
```

**Add these variants** to `IpcResponse` (per D-09, D-10):

```rust
    Identicon {
        fingerprint_hex: String,
        identicon: String,
    },
    WordPhrase {
        words: Vec<String>,
        phrase: String,
    },
```

**serde derive pattern** — preserve the `#[serde(rename_all = "snake_case", tag = "type")]` attribute on `IpcResponse`. The new variants serialize to `{"type": "identicon", ...}` and `{"type": "word_phrase", ...}` automatically.

**Imports** — no change needed; `Serialize` and `Deserialize` already imported (line 1 of ipc.rs).

---

### `crates/periphore-config/src/schema.rs` (model/config — extend existing)

**Analog:** `crates/periphore-config/src/schema.rs` lines 31-43 — `LoggingConfig` struct with custom `Default` impl. This is the closest match for `IdentityConfig` (a config section that has a non-trivial default).

**`LoggingConfig` pattern to copy** (lines 31-43):

```rust
// Analog from schema.rs lines 31-43:
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
```

**Apply same pattern for `IdentityConfig`**:

```rust
/// Identity display configuration.
#[derive(Debug, Deserialize)]
pub struct IdentityConfig {
    /// Show identicon on startup and in IPC GetIdenticon responses.
    /// Set to false for headless or automated setups (SEC-04).
    ///
    /// Note: this field contains an underscore, which means it cannot be set via
    /// the PERIPHORE_IDENTITY_SHOW_IDENTICON env var (Figment split("_") would map
    /// to identity.show.identicon — 3 levels). Use the TOML config file instead.
    pub show_identicon: bool,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self { show_identicon: true }
    }
}
```

**Add field to `Config` struct** — copy the `#[serde(default)]` pattern from `Config` lines 7-19:

```rust
// In Config struct (lines 7-19 show the pattern):
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub peers: Vec<PeerConfig>,
    #[serde(default)]
    pub topology: TopologyConfig,
    #[serde(default)]
    pub capture: CaptureConfig,
    // ADD:
    #[serde(default)]
    pub identity: IdentityConfig,
}
```

**`pub use` in `lib.rs`** — add `IdentityConfig` to the re-export line in `crates/periphore-config/src/lib.rs` line 15:

```rust
// Current (lib.rs line 15):
pub use schema::{CaptureConfig, Config, DaemonConfig, LoggingConfig, PeerConfig, TopologyConfig};
// Change to:
pub use schema::{CaptureConfig, Config, DaemonConfig, IdentityConfig, LoggingConfig, PeerConfig, TopologyConfig};
```

---

### `crates/periphored/src/main.rs` (controller, request-response — extend existing)

**Analog:** `crates/periphored/src/main.rs` lines 106-145 — the `select!` IPC dispatch block.

**Config loading pattern to copy** (lines 44-46 and 48-51) — apply same pattern for identity startup:

```rust
// Analog: config load pattern (main.rs lines 44-46):
let config = periphore_config::load(args.config.as_deref())
    .map_err(|e| anyhow::anyhow!("failed to load config: {e}"))?;

tracing::info!(
    log_level = %config.logging.level,
    "periphored starting"
);
```

**Identity startup insertion point** — after config load (line 46), before IPC channel creation (line 75). Insert:

```rust
// After config load, before IPC channel:
let key_path = periphore_identity::default_key_path()
    .ok_or_else(|| anyhow::anyhow!("cannot determine key storage path"))?;
let identity = periphore_identity::IdentityStore::load_or_create(&key_path)
    .map_err(|e| anyhow::anyhow!("identity error: {e}"))?;
tracing::info!(
    fingerprint = %identity.fingerprint_hex(),
    "identity loaded"
);
```

**GetStatus dispatch pattern** (lines 108-115) — `GetIdenticon` and `GetWordPhrase` move out of `send_ok()` and into the `select!` arms using this exact pattern:

```rust
// Analog: GetStatus arm (main.rs lines 108-115):
Some(IpcCommand::GetStatus { responder }) => {
    tracing::debug!("IPC: GetStatus");
    let _ = responder.send(IpcResponse::Status {
        running:     true,
        fingerprint: None, // Phase 2: real Ed25519 fingerprint
    });
}
```

**New dispatch arms to add** — using the same `Some(IpcCommand::...)` pattern:

```rust
Some(IpcCommand::GetIdenticon { responder, .. }) => {
    tracing::debug!("IPC: GetIdenticon");
    let _ = responder.send(IpcResponse::Identicon {
        fingerprint_hex: identity.fingerprint_hex(),
        identicon: identity.identicon(),
    });
}
Some(IpcCommand::GetWordPhrase { responder, .. }) => {
    tracing::debug!("IPC: GetWordPhrase");
    let words = identity.word_phrase();
    let phrase = words.join(" ");
    let _ = responder.send(IpcResponse::WordPhrase { words, phrase });
}
```

**`GetStatus` fingerprint field** — update the existing `GetStatus` arm (line 114) from `None` to:

```rust
fingerprint: Some(identity.fingerprint_hex()),
```

**Remove from `send_ok()`** — delete the existing stub arms for `GetIdenticon` and `GetWordPhrase` from `send_ok()` (current lines 202-207):

```rust
// Remove these stubs:
IpcCommand::GetIdenticon { responder, .. } => {
    let _ = responder.send(IpcResponse::Ok);
}
IpcCommand::GetWordPhrase { responder, .. } => {
    let _ = responder.send(IpcResponse::Ok);
}
```

---

### `crates/periphored/Cargo.toml` (config — add dep)

**Analog:** `crates/periphored/Cargo.toml` lines 18-26 (existing `[dependencies]` section).

**Current pattern** (lines 18-26):

```toml
[dependencies]
periphore-config   = { workspace = true }
periphore-ipc      = { workspace = true }
periphore-protocol = { workspace = true }
tokio              = { workspace = true }
clap               = { workspace = true }
tracing            = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow             = { workspace = true }
```

**Add one line** after `periphore-config`:

```toml
periphore-identity = { workspace = true }
```

---

### `Cargo.toml` (workspace — add rand_core + ed25519-dalek features)

**Analog:** `Cargo.toml` lines 40-43 (existing external dependency block).

**Current relevant lines** (lines 40-43):

```toml
thiserror         = { version = "2.0" }
ed25519-dalek     = { version = "2.2" }
sha2              = { version = "0.10" }
anyhow            = { version = "1.0" }
```

**Changes:**

1. Add `rand_core` after `sha2`:

```toml
rand_core         = { version = "0.6", features = ["getrandom"] }
```

2. Update `ed25519-dalek` to enable the `rand_core` feature at workspace level:

```toml
ed25519-dalek     = { version = "2.2", features = ["rand_core"] }
```

---

## Shared Patterns

### thiserror Error Type Derivation
**Source:** `crates/periphore-config/src/lib.rs` lines 23-38 (manual error type) + RESEARCH.md §Implementation Strategy (thiserror pattern from research)
**Apply to:** `periphore-identity/src/lib.rs`

```rust
// Manual pattern in config crate (lib.rs lines 23-38) — but identity uses thiserror:
#[derive(Debug)]
pub struct ConfigError(figment::Error);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "config error: {}", self.0)
    }
}
impl std::error::Error for ConfigError {}
impl From<figment::Error> for ConfigError {
    fn from(e: figment::Error) -> Self { Self(e) }
}
```

Identity uses `thiserror` instead (per D-17 — consistent with workspace convention):
```rust
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("key file is corrupt (expected 32 bytes, got {0})")]
    CorruptKeyFile(usize),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("no valid data directory found for this user")]
    NoDataDir,
}
```

### Workspace Dep Reference
**Source:** Any crate `Cargo.toml` — e.g., `crates/periphore-identity/Cargo.toml` lines 17-20
**Apply to:** All Cargo.toml changes

```toml
# Correct pattern — always { workspace = true }, never bare version strings:
thiserror = { workspace = true }
# NOT: thiserror = "2.0"
```

### tracing Structured Logging
**Source:** `crates/periphored/src/main.rs` lines 48-51, 61, 87
**Apply to:** `periphore-identity/src/lib.rs` (first-run log), `periphored/src/main.rs` (identity loaded log)

```rust
// Named field + message pattern (main.rs lines 48-51):
tracing::info!(
    log_level = %config.logging.level,
    "periphored starting"
);

// Inline interpolation pattern (main.rs line 61):
tracing::info!(path = %socket_path.display(), "IPC socket path");

// For identity crate (inside load_or_create, first-run path):
tracing::info!("Generated new identity: {}", fingerprint_hex);
// For periphored startup (after load_or_create returns):
tracing::info!(fingerprint = %identity.fingerprint_hex(), "identity loaded");
```

### anyhow at Daemon Boundary
**Source:** `crates/periphored/src/main.rs` lines 45-46, 83-84
**Apply to:** `periphored/src/main.rs` identity integration

```rust
// Pattern: .map_err(|e| anyhow::anyhow!("context: {e}"))? at daemon boundary
// (main.rs line 45-46):
let config = periphore_config::load(args.config.as_deref())
    .map_err(|e| anyhow::anyhow!("failed to load config: {e}"))?;

// (main.rs line 83-84):
.map_err(|e| anyhow::anyhow!("IPC server error: {e}"))
```

### Unix File Permissions (0600)
**Source:** `crates/periphore-ipc/src/server.rs` lines 27-46
**Apply to:** `periphore-identity/src/lib.rs` (`load_or_create` first-run file creation)

```rust
// server.rs uses post-creation set_permissions:
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(socket_path, fs::Permissions::from_mode(0o600))?;
}

// Identity crate uses OpenOptionsExt::mode() instead (atomic, no race window — RESEARCH.md Pitfall 6):
use std::os::unix::fs::OpenOptionsExt;
let mut file = std::fs::OpenOptions::new()
    .write(true)
    .create_new(true)
    .mode(0o600)
    .open(path)?;
file.write_all(&seed)?;
```

### serde Tag Enum Pattern
**Source:** `crates/periphore-protocol/src/ipc.rs` lines 7-9 and 41-43
**Apply to:** `IpcResponse` new variants (must preserve existing serde attributes)

```rust
// Existing attribute must be preserved (ipc.rs lines 41-43):
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum IpcResponse {
```

New variants automatically serialize as `{"type": "identicon", ...}` and `{"type": "word_phrase", ...}` due to `rename_all = "snake_case"`.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `crates/periphore-identity/tests/identity.rs` | test | — | No integration test files exist in any crate yet; first test file in workspace |
| `crates/periphore-identity/src/bip39.rs` | utility | transform | No inline static wordlist module exists in codebase |

---

## Metadata

**Analog search scope:** `crates/*/src/*.rs`, `crates/*/Cargo.toml`, `Cargo.toml`
**Files scanned:** 13 (main.rs, ipc.rs×2, server.rs, lib.rs×4, schema.rs, peer.rs, Cargo.toml×3)
**Pattern extraction date:** 2026-04-22
