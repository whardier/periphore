# Phase 3: Configuration & Trust Persistence - Pattern Map

**Mapped:** 2026-04-24
**Files analyzed:** 9 (new/modified)
**Analogs found:** 9 / 9

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/periphore-trust/Cargo.toml` | config | — | `crates/periphore-identity/Cargo.toml` | exact |
| `crates/periphore-trust/src/lib.rs` | library entry | — | `crates/periphore-identity/src/lib.rs` | exact |
| `crates/periphore-trust/src/store.rs` | service | file-I/O + CRUD | `crates/periphore-identity/src/lib.rs` (IdentityStore) | role-match |
| `crates/periphore-trust/tests/trust.rs` | test | — | `crates/periphore-identity/tests/identity.rs` | exact |
| `crates/periphore-config/src/schema.rs` | model | — | `crates/periphore-config/src/schema.rs` (PeerConfig, TopologyConfig) | exact (modify) |
| `crates/periphore-config/tests/config.rs` | test | — | `crates/periphore-config/tests/config.rs` | exact (extend) |
| `crates/periphored/src/main.rs` | controller | event-driven | `crates/periphored/src/main.rs` (existing named arms) | exact (modify) |
| `crates/periphored/Cargo.toml` | config | — | `crates/periphored/Cargo.toml` | exact (modify) |
| `Cargo.toml` (workspace root) | config | — | `Cargo.toml` `[workspace.dependencies]` section | exact (modify) |

---

## Pattern Assignments

### `crates/periphore-trust/Cargo.toml` (new crate manifest)

**Analog:** `crates/periphore-identity/Cargo.toml`

**Full analog** (lines 1-27):
```toml
[package]
name = "periphore-identity"
version.workspace    = true
edition.workspace    = true
authors.workspace    = true
license.workspace    = true
repository.workspace = true
publish.workspace    = true

[lib]
doctest = false
test    = false

[lints]
workspace = true

[dependencies]
ed25519-dalek = { workspace = true, features = ["rand_core"] }
sha2          = { workspace = true }
serde         = { workspace = true }
rand_core     = { workspace = true }
thiserror     = { workspace = true }
directories   = { workspace = true }
tracing       = { workspace = true }

[dev-dependencies]
tempfile = "3"
```

**Adaptation for periphore-trust:** Replace name, remove identity-specific deps (`ed25519-dalek`, `sha2`, `rand_core`), add `toml = { workspace = true }` and `tempfile = { workspace = true }` to `[dependencies]`. The `[dev-dependencies]` section needs `tempfile` as well but it will already be in `[dependencies]` — omit the duplicate or keep only `[dependencies]` entry. The `[lib] test = false` pattern is identical.

**Key difference from periphore-config/Cargo.toml:** No `[features]` block. No `figment`. No optional `clap`. Trust crate has no feature flags.

---

### `crates/periphore-trust/src/lib.rs` (module re-export entry point)

**Analog:** `crates/periphore-identity/src/lib.rs` (top-of-file module declarations and error type)

**Module declaration pattern** (lines 1-17):
```rust
//! periphore-identity: Ed25519 keypair lifecycle, SHA-256 fingerprints,
//! identicon rendering (Drunken Bishop), and BIP39 word phrases.
//!
//! SEC-01: load_or_create() — persistent Ed25519 identity.

mod bip39;

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
// ... other imports
use thiserror::Error;
```

**Error type pattern** (lines 19-31):
```rust
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
```

**Adaptation for periphore-trust/src/lib.rs:** `lib.rs` re-exports from `store` module. Pattern: `pub mod store; pub use store::{TrustStore, TrustedPeer, TrustError};`. The `TrustError` enum lives in `store.rs` or can be declared in `lib.rs` and re-exported. Prefer declaring in `store.rs` alongside the struct it serves.

**`default_key_path` → `default_trust_path` pattern** (lines 239-250):
```rust
/// Return the platform-appropriate path for the identity key file.
///
/// Linux:  `$XDG_DATA_HOME/periphore/key`  (default: `~/.local/share/periphore/key`)
/// macOS:  `~/Library/Application Support/periphore/key`
///
/// Returns `None` when `ProjectDirs::from` cannot find a home directory
pub fn default_key_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "periphore")
        .map(|dirs| dirs.data_dir().join("key"))
}
```

**Adaptation:** Copy verbatim as `pub fn default_trust_path() -> Option<PathBuf>`, changing `.join("key")` to `.join("trusted.toml")`. Same `ProjectDirs::from("", "", "periphore")` call — identical platform-correct path resolution.

---

### `crates/periphore-trust/src/store.rs` (TrustStore implementation)

**Analog:** `crates/periphore-identity/src/lib.rs` — `IdentityStore` struct + `load_or_create` pattern

**Load pattern** (lines 57-105):
```rust
pub fn load_or_create(path: &Path) -> Result<Self, IdentityError> {
    if path.exists() {
        // --- Load existing key ---
        let bytes = std::fs::read(path)?;
        if bytes.len() != 32 {
            return Err(IdentityError::CorruptKeyFile(bytes.len()));
        }
        // ...
        Ok(Self { keypair, fingerprint })
    } else {
        // --- Generate new keypair (first run) ---
        // Create parent directory if it does not exist (D-04).
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // ...
        Ok(Self { keypair, fingerprint })
    }
}
```

**Adaptation for TrustStore::load:** Same `if path.exists()` branch. On exists: `std::fs::read_to_string(path)` + `toml::from_str(&content).map_err(|e| TrustError::CorruptCacheFile(e.to_string()))`. On missing: return `Ok(Self { cache: TrustCache { trusted: vec![] } })` — no file creation on load (creation happens on first `add_trusted`). The `create_dir_all` guard moves to the `save` method instead.

**Tracing pattern** (line 101):
```rust
tracing::info!("Generated new identity: {}", fingerprint_hex);
```

Copy tracing pattern: `tracing::info!(fingerprint = %fingerprint, "fingerprint trusted and cached")` for add operations; `tracing::warn!` for conflict/error cases.

---

### `crates/periphore-trust/tests/trust.rs` (integration tests)

**Analog:** `crates/periphore-identity/tests/identity.rs`

**File header pattern** (lines 1-9):
```rust
//! Integration and unit tests for periphore-identity.
//! All tests live here because [lib] test = false in Cargo.toml (Phase 1 D-07).
//!
//! SEC-01 tests: test_first_run_creates_key_file, test_load_after_create_is_identical, ...

use std::fs;

use periphore_identity::{IdentityError, IdentityStore};
```

**Adaptation:** `use periphore_trust::{TrustError, TrustStore};` — same import style. Use `tempfile::tempdir()` for all test paths (same pattern).

**Test structure pattern** (lines 19-33):
```rust
#[test]
fn test_first_run_creates_key_file() {
    // load_or_create on a non-existent path must create the key file.
    let dir = tempfile::tempdir().expect("temp dir");
    let key_path = dir.path().join("key");

    assert!(!key_path.exists(), "key must not exist before first run");
    let _store = IdentityStore::load_or_create(&key_path)
        .expect("load_or_create must succeed on first run");
    assert!(key_path.exists(), "key file must exist after load_or_create");
    // ...
}
```

**Adaptation:** Replace `key_path.join("key")` with `dir.path().join("trusted.toml")`. The `_dir` keep-alive pattern (holding `TempDir` to prevent cleanup) is identical. Error match patterns: `Err(TrustError::CorruptCacheFile(_))` mirrors `Err(IdentityError::CorruptKeyFile(16))`.

---

### `crates/periphore-config/src/schema.rs` (modify: add PeerConfig.name + MonitorConfig + TopologyConfig.monitors)

**Analog:** `crates/periphore-config/src/schema.rs` — current `PeerConfig` and `TopologyConfig`

**Current PeerConfig** (lines 48-57):
```rust
/// Per-peer configuration. Repeated as [[peer]] in TOML.
#[derive(Debug, Deserialize, Default)]
pub struct PeerConfig {
    /// Expected peer fingerprint (hex string). Optional -- if set, connection from non-matching
    /// fingerprint is refused (hard config enforcement, Phase 3 SEC-06).
    pub fingerprint: Option<String>,
    /// Manual host for connecting to this peer (Phase 6 NET-03).
    pub host: Option<String>,
    /// TCP port override for this peer.
    pub port: Option<u16>,
}
```

**Add after `fingerprint` field:** Insert `pub name: Option<String>` with doc comment. Doc comment MUST state it is local-only and NOT sent over the wire (D-11 mandate).

**Current TopologyConfig** (lines 59-63):
```rust
/// Monitor topology configuration (Phase 8 fills this in).
#[derive(Debug, Deserialize, Default)]
pub struct TopologyConfig {
    // Edge layout, alignment preferences -- populated in Phase 8.
}
```

**Replace with:** Add `pub monitors: Vec<MonitorConfig>` field decorated with `#[serde(default, rename = "monitor")]`. Add new `MonitorConfig` struct before `TopologyConfig`. All fields `Option<T>` with `Default` derive. Update the doc comment to reference Phase 3 (monitors) and Phase 8 (edge config).

**Serde attribute pattern from existing code** (line 9): `#[serde(default)]` — use same pattern on `monitors` field. Add `rename = "monitor"` to map `[[topology.monitor]]` TOML to the `monitors` Vec field.

**Critical constraint** (lines 1-6): Config structs do NOT derive `Serialize`. `MonitorConfig` must only derive `Debug, Deserialize, Default`. Never add `Serialize` to any config struct.

---

### `crates/periphore-config/tests/config.rs` (extend: new tests for CFG-02, CFG-03)

**Analog:** `crates/periphore-config/tests/config.rs` — existing test pattern

**Test pattern** (lines 85-91):
```rust
#[test]
fn peer_config_vec_defaults_to_empty() {
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    let config = load(None).expect("default config");
    assert!(config.peers.is_empty(), "peers should default to empty vec");
}
```

**TOML-write-and-load test pattern** (lines 37-49):
```rust
#[test]
fn toml_file_overrides_defaults() {
    let _guard = ENV_MUTEX.lock().unwrap();
    clear_periphore_env();

    let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
    writeln!(tmp, "[logging]").unwrap();
    writeln!(tmp, r#"level = "debug""#).unwrap();

    let config = load(Some(tmp.path())).expect("should load with TOML file");
    assert_eq!(config.logging.level, "debug");
}
```

**Adaptation for CFG-02 (PeerConfig.name):** Write TOML with `[[peer]]` block including `name = "work-mac"`. Load. Assert `config.peers[0].name == Some("work-mac".to_owned())`.

**Adaptation for CFG-03 (MonitorConfig):** Write TOML with `[[topology.monitor]]` entries. Load. Assert `config.topology.monitors.len()`, field values for `id`, `name`, `width`, `height`. Add test for empty-monitors default (no `[[topology.monitor]]` in TOML → `monitors` is empty vec).

**Mandatory boilerplate:** Every new test must acquire `ENV_MUTEX.lock()` first and call `clear_periphore_env()` — copy from existing tests verbatim.

---

### `crates/periphored/src/main.rs` (promote AcceptFingerprint/RejectFingerprint from send_ok to named arms)

**Analog:** `crates/periphored/src/main.rs` — existing named match arms in `tokio::select!`

**Named arm pattern** (lines 132-163):
```rust
Some(IpcCommand::GetStatus { responder }) => {
    tracing::debug!("IPC: GetStatus");
    let _ = responder.send(IpcResponse::Status {
        running:     true,
        fingerprint: Some(identity.fingerprint_hex()),
    });
}
Some(IpcCommand::InjectInputEvent { event, responder }) => {
    // D-19: InjectInputEvent is the IPC test backbone.
    // Phase 9 wires this to real capture/inject; for now, log and ack.
    tracing::debug!(?event, "IPC: InjectInputEvent");
    let _ = responder.send(IpcResponse::Ok);
}
```

**Error-surfacing pattern** (lines 57-58):
```rust
let config = periphore_config::load(args.config.as_deref())
    .map_err(|e| anyhow::anyhow!("failed to load config: {e}"))?;
```

Copy this `map_err(|e| anyhow::anyhow!("...{e}"))` pattern for TrustStore load at startup.

**Startup initialization pattern** (lines 66-75):
```rust
let key_path = periphore_identity::default_key_path()
    .ok_or_else(|| anyhow::anyhow!("cannot determine identity key storage path"))?;
let identity = periphore_identity::IdentityStore::load_or_create(&key_path)
    .map_err(|e| anyhow::anyhow!("identity error: {e}"))?;
tracing::info!(
    fingerprint = %identity.fingerprint_hex(),
    "identity loaded"
);
```

**Adaptation:** Add analogous block for TrustStore immediately after identity load block. `default_trust_path()` with `.ok_or_else(|| anyhow::anyhow!("cannot determine trust cache path"))`. `TrustStore::load(&trust_path).map_err(|e| anyhow::anyhow!("trust cache error: {e}"))`.

**IpcCommand field names for the stubs being promoted** (lines 225-231 of send_ok):
```rust
IpcCommand::AcceptFingerprint { responder, .. } => {
    let _ = responder.send(IpcResponse::Ok);
}
IpcCommand::RejectFingerprint { responder, .. } => {
    let _ = responder.send(IpcResponse::Ok);
}
```

The `..` in the stub ignores `fingerprint`. The new named arms must bind `fingerprint` explicitly: `IpcCommand::AcceptFingerprint { fingerprint, responder }`.

**Tracing field style** (line 60): `tracing::info!(fingerprint = %fingerprint, "IPC: AcceptFingerprint")` — structured field with `%` for Display formatting. Match existing style in the loop.

**send_ok cleanup:** After adding named arms, remove `AcceptFingerprint` and `RejectFingerprint` arms from `send_ok()` (lines 225-231). Rust exhaustiveness checking will catch any missed cleanup.

---

### `crates/periphored/Cargo.toml` (add periphore-trust dependency)

**Analog:** `crates/periphored/Cargo.toml` existing `[dependencies]` section (lines 17-28):
```toml
[dependencies]
periphore-config   = { workspace = true }
periphore-identity = { workspace = true }
periphore-ipc      = { workspace = true }
periphore-protocol = { workspace = true }
tokio              = { workspace = true }
clap               = { workspace = true }
tracing            = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow             = { workspace = true }

[dev-dependencies]
tempfile = "3"
```

**Adaptation:** Add `periphore-trust = { workspace = true }` to `[dependencies]`. Keep alphabetical-ish ordering. No change to `[dev-dependencies]`.

---

### `Cargo.toml` (workspace root — add workspace deps)

**Analog:** `Cargo.toml` `[workspace.dependencies]` internal crate entries (lines 16-26):
```toml
# Internal library crates — path + version per D-03; never bare path refs
periphore-protocol = { path = "crates/periphore-protocol", version = "0.1.0" }
periphore-config   = { path = "crates/periphore-config",   version = "0.1.0" }
periphore-identity = { path = "crates/periphore-identity", version = "0.1.0" }
periphore-core     = { path = "crates/periphore-core",     version = "0.1.0" }
periphore-ipc      = { path = "crates/periphore-ipc",      version = "0.1.0" }
periphore-net      = { path = "crates/periphore-net",      version = "0.1.0" }
periphore-capture  = { path = "crates/periphore-capture",  version = "0.1.0" }
periphore-inject   = { path = "crates/periphore-inject",   version = "0.1.0" }
periphore-cli      = { path = "crates/periphore-cli",      version = "0.1.0" }
```

**External dep pattern** (lines 30-46):
```toml
# External dependencies — versions verified 2026-04-22 via cargo registry
tokio             = { version = "1.52", features = ["net", "macros", "rt-multi-thread", "signal", "io-util", "sync", "time"] }
// ...
thiserror         = { version = "2.0" }
```

**Adaptation:** Add three entries:
1. Internal: `periphore-trust = { path = "crates/periphore-trust", version = "0.1.0" }` — after `periphore-identity`, before `periphore-core` (build order: protocol → config + identity → **trust** → core + ipc + cli → ...).
2. External: `toml = { version = "0.8", features = ["display"] }` — in the external deps block.
3. External: `tempfile = { version = "3" }` — in the external deps block.

Note: `periphore-identity/Cargo.toml` uses bare `tempfile = "3"` in dev-deps (not workspace = true). Phase 3 promotes `tempfile` to a workspace dep so `periphore-trust` can use `{ workspace = true }` in its regular `[dependencies]`. After this change, `periphore-identity` and `periphore-config` should also switch their `tempfile = "3"` dev-deps to `tempfile = { workspace = true }` for consistency — but that is an optional cleanup, not required for correctness.

---

## Shared Patterns

### thiserror Error Enum
**Source:** `crates/periphore-identity/src/lib.rs` lines 19-31
**Apply to:** `crates/periphore-trust/src/store.rs` (TrustError enum)
```rust
#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("key file is corrupt (expected 32 bytes, got {0})")]
    CorruptKeyFile(usize),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("no valid data directory found for this user")]
    NoDataDir,
}
```
All variants use string interpolation in `#[error(...)]`. `#[from]` on `io::Error` enables `?` propagation from `std::io` operations. Named-field variants use struct-like syntax in the error message. This is the exact pattern for `TrustError`.

### anyhow at Daemon Boundary
**Source:** `crates/periphored/src/main.rs` lines 57-58, 68-71
**Apply to:** `crates/periphored/src/main.rs` TrustStore load block
```rust
let config = periphore_config::load(args.config.as_deref())
    .map_err(|e| anyhow::anyhow!("failed to load config: {e}"))?;
// ...
let identity = periphore_identity::IdentityStore::load_or_create(&key_path)
    .map_err(|e| anyhow::anyhow!("identity error: {e}"))?;
```
Library errors (`IdentityError`, `TrustError`) are converted to `anyhow::Error` at the daemon boundary. Library crates never use `anyhow` internally.

### ProjectDirs Path Resolution
**Source:** `crates/periphore-identity/src/lib.rs` lines 247-250
**Apply to:** `crates/periphore-trust/src/lib.rs` (`default_trust_path` function)
```rust
pub fn default_key_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "periphore")
        .map(|dirs| dirs.data_dir().join("key"))
}
```
`ProjectDirs::from("", "", "periphore")` is the project-standard call. `.data_dir()` gives the XDG data home on Linux and `~/Library/Application Support/periphore` on macOS. Copy verbatim, change `.join("key")` to `.join("trusted.toml")`.

### Test Tempdir Keep-Alive
**Source:** `crates/periphore-identity/tests/identity.rs` lines 20-33
**Apply to:** `crates/periphore-trust/tests/trust.rs`
```rust
let dir = tempfile::tempdir().expect("temp dir");
let key_path = dir.path().join("key");
// ... use key_path ...
// dir is kept in scope to prevent early cleanup
```
Always bind the `TempDir` to a named variable. Never use `tempfile::tempdir().path()` directly (drops immediately). The `_dir` convention signals intentional hold.

### ENV_MUTEX in Config Tests
**Source:** `crates/periphore-config/tests/config.rs` lines 17-24
**Apply to:** New tests in `crates/periphore-config/tests/config.rs`
```rust
static ENV_MUTEX: Mutex<()> = Mutex::new(());

fn clear_periphore_env() {
    unsafe { std::env::remove_var("PERIPHORE_LOGGING_LEVEL") };
}

// In every test:
let _guard = ENV_MUTEX.lock().unwrap();
clear_periphore_env();
```
Every config test acquires this mutex first. New CFG-02 and CFG-03 tests follow the same pattern exactly — no env vars needed for peer/topology tests, but the mutex ensures test isolation.

### IpcCommand Named Arm with Error Response
**Source:** `crates/periphored/src/main.rs` lines 132-143
**Apply to:** `crates/periphored/src/main.rs` AcceptFingerprint arm
```rust
Some(IpcCommand::GetStatus { responder }) => {
    tracing::debug!("IPC: GetStatus");
    let _ = responder.send(IpcResponse::Status { ... });
}
```
The `let _ = responder.send(...)` pattern discards the `Result` because a closed receiver (client disconnected) is not an error worth propagating. Use `tracing::info!` (not `debug!`) for AcceptFingerprint/RejectFingerprint since these are user-initiated trust decisions.

---

## No Analog Found

All files have close analogs. No entries.

---

## Metadata

**Analog search scope:** `crates/periphore-identity/`, `crates/periphore-config/`, `crates/periphored/`, workspace `Cargo.toml`
**Files scanned:** 9 source files read directly
**Pattern extraction date:** 2026-04-24
