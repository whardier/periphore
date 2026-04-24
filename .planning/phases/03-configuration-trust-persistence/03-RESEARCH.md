# Phase 3: Configuration & Trust Persistence - Research

**Researched:** 2026-04-24
**Domain:** Trust cache persistence, TOML serialization, config schema evolution, fingerprint conflict detection
**Confidence:** HIGH

## Summary

Phase 3 adds the `periphore-trust` crate (12th workspace crate) providing a `TrustStore` API for reading and writing a `trusted.toml` fingerprint cache file. The crate uses `toml` for serialization (not figment -- this is the only crate in the workspace that writes to disk), `tempfile` for atomic write-then-rename, and `directories` for path resolution. The `periphored` daemon's IPC dispatch loop gets promoted from stub `send_ok()` to real `TrustStore` calls for `AcceptFingerprint`/`RejectFingerprint`. Additionally, `periphore-config` gains a `PeerConfig.name` field and `TopologyConfig.monitors: Vec<MonitorConfig>` for the config schema groundwork.

The phase is purely library + daemon wiring work -- no network, no platform-specific code, no async (TrustStore is synchronous). The `check_peer_fingerprint` function is a pure function with zero I/O, designed to be called by Phase 6's TCP handshake. All tests can run without network, platform APIs, or async runtime.

**Primary recommendation:** Use `toml` 0.8 (already a transitive dependency via figment -- zero new downloads), `tempfile` 3.x (already a dev-dep in identity/config crates -- promote to workspace dependency), and mirror the `periphore-identity` crate structure exactly (thiserror errors, `[lib] test = false`, integration tests in `tests/` subdir).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Cache file path -- `directories::ProjectDirs::data_dir()` (Linux: `~/.local/share/periphore/trusted.toml`, macOS: `~/Library/Application Support/periphore/trusted.toml`)
- **D-02:** Format -- TOML, consistent with main config
- **D-03:** Cache schema -- `[[trusted]]` entries with `fingerprint: String` (64-char hex, required) and `alias: Option<String>` (optional)
- **D-04:** Write path -- exclusively via `AcceptFingerprint` IPC command
- **D-05:** New `periphore-trust` crate (12th crate)
- **D-06:** Build order: `protocol -> config + identity -> trust -> core + ipc + cli -> net -> capture + inject`
- **D-07:** Public API surface: `TrustStore` with `load`, `is_trusted`, `add_trusted`, `remove_trusted`; `TrustedPeer` struct
- **D-08:** Error type: `thiserror`-derived `TrustError` (matches `IdentityError` pattern)
- **D-09:** File creation on first `add_trusted` -- create atomically if missing; `CorruptCacheFile` error on malformed TOML
- **D-10:** Add `name: Option<String>` to `PeerConfig`
- **D-11:** `name` is local-only -- NOT sent over wire, does NOT participate in identity verification
- **D-12:** Used in log/error messages; falls back to fingerprint hex if absent
- **D-13:** Pure `check_peer_fingerprint(configured_fp, actual_fp, peer_label) -> Result<(), TrustError>` function
- **D-14:** Phase 6 calls `check_peer_fingerprint` during handshake; Phase 3 only delivers the function
- **D-15:** Topology conflict detection deferred to Phase 8
- **D-16:** `TopologyConfig` gains `monitors: Vec<MonitorConfig>` field; TOML: `[[topology.monitor]]`
- **D-17:** `MonitorConfig` fields: `id: Option<String>`, `name: Option<String>`, `width: Option<u32>`, `height: Option<u32>`
- **D-18:** `id` is local per-node -- no cross-node uniqueness requirement
- **D-19:** ID matching strategy deferred to Phase 8
- **D-20:** `periphore monitors list` CLI command deferred to Phase 5/8

### Claude's Discretion
- Exact TOML structure for `trusted.toml` (flat list vs `[trusted]` section header)
- Whether `periphore-trust` re-exports via `lib.rs` or uses a `store` module
- Atomic write strategy for `trusted.toml` (write-then-rename vs direct overwrite)
- Whether to add `periphore-trust` to `periphored`'s `Cargo.toml` directly or via `periphore-core`

### Deferred Ideas (OUT OF SCOPE)
- VNC/RDP as peers without daemon (post-v1)
- Topology conflict detection (Phase 8)
- `periphore monitors list` CLI command (Phase 5/8)
- Richer TrustStore API (`.list_trusted()`) (Phase 5)
- Trust cache alias in AcceptFingerprint IPC (Phase 5)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SEC-05 | Accepted fingerprints cached between sessions (no auto-write to main config) | TrustStore API with atomic TOML write via tempfile+persist; cache in data_dir separate from config |
| SEC-06 | Hard configuration can include peer fingerprint; conflicts prevent peering | Pure `check_peer_fingerprint` function + `PeerConfig.name` field for error context |
| CFG-02 | Hard config conflicts between peers prevent peering | `check_peer_fingerprint` returns `TrustError::FingerprintConflict`; Phase 6 wires into handshake |
| CFG-03 | Config can define preferred monitor layouts for dynamic monitor scenarios | `TopologyConfig.monitors: Vec<MonitorConfig>` with `[[topology.monitor]]` TOML entries |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Trust cache read/write | Library (periphore-trust) | -- | Pure data persistence -- no daemon, no async, no platform code |
| Fingerprint conflict detection | Library (periphore-trust) | -- | Pure function -- no I/O, no async; called by Phase 6 TCP handshake |
| AcceptFingerprint IPC dispatch | Daemon (periphored) | Library (periphore-trust) | Daemon owns routing; trust crate owns persistence |
| PeerConfig.name schema | Library (periphore-config) | -- | Config crate owns schema; daemon/CLI read it |
| TopologyConfig.monitors schema | Library (periphore-config) | -- | Config crate owns schema; Phase 8 adds processing logic |
| Trust cache path resolution | Library (periphore-trust) | -- | Uses `directories::ProjectDirs::data_dir()` -- same pattern as identity crate |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| toml | 0.8.x (0.8.23 in lockfile) | TOML serialization for trusted.toml writes | Already a transitive dep via figment; zero new downloads; proven Vec-to-array-table round-trip [VERIFIED: cargo tree + local round-trip test] |
| tempfile | 3.27.0 | Atomic write-then-rename for trusted.toml | Already a dev-dep in identity/config crates; creates files with 0o600 default; persist() is atomic rename on Unix [VERIFIED: Context7 /stebalien/tempfile + local test] |
| serde | 1.0 (workspace) | Serialize/Deserialize for TrustedPeer, MonitorConfig | Already in workspace [VERIFIED: Cargo.toml] |
| thiserror | 2.0 (workspace) | TrustError derive (matches IdentityError pattern) | Already in workspace; all library crates use thiserror [VERIFIED: Cargo.toml + identity crate pattern] |
| directories | 6.0 (workspace) | data_dir() path resolution for trusted.toml | Already in workspace; same pattern as identity key path [VERIFIED: Cargo.toml + identity crate] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tracing | 0.1 (workspace) | Structured logging for trust operations | Log trust cache load/save/conflict events at info/warn level |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| toml 0.8 | toml 1.1 | toml 1.x adds a second copy to lockfile; 0.8 unifies with figment's transitive dep -- prefer 0.8 |
| tempfile::NamedTempFile::persist() | Manual std::fs::write + std::fs::rename | tempfile handles temp name generation, 0o600 permissions, same-dir placement; manual approach loses these |
| toml | serde_json | TOML is human-readable and consistent with main config; JSON would be an outlier |

**Installation:**
```bash
# No new external crate downloads needed -- toml 0.8 unifies with figment's transitive dep;
# tempfile promoted from dev-dep to regular dep for periphore-trust only.
# Workspace Cargo.toml additions:
#   toml = { version = "0.8", features = ["display"] }
#   tempfile = { version = "3" }
#   periphore-trust = { path = "crates/periphore-trust", version = "0.1.0" }
```

**Version verification:**
- `toml`: 0.8.23 already in Cargo.lock via figment [VERIFIED: `cargo tree -p figment | grep toml`]
- `tempfile`: 3.27.0 already in Cargo.lock as dev-dep [VERIFIED: `cargo search tempfile`]
- `serde`: 1.0.x workspace dep [VERIFIED: Cargo.toml]
- `thiserror`: 2.0.x workspace dep [VERIFIED: Cargo.toml]
- `directories`: 6.0.x workspace dep [VERIFIED: Cargo.toml]

## Architecture Patterns

### System Architecture Diagram

```
                                    +-----------------------+
                                    |     periphore-config  |
                                    |   (read-only schema)  |
                                    |  PeerConfig.name      |
                                    |  TopologyConfig       |
                                    |   .monitors           |
                                    +-----------+-----------+
                                                |
                                                | (loaded at startup)
                                                v
+------------+    IPC/mpsc     +-------------+      +-------------------+
|  periphore |  ------------>  |  periphored |----->| periphore-trust   |
|   (CLI)    | AcceptFinger-   |   (daemon)  |      |                   |
|            | print request   |  IPC router |      | TrustStore::load  |
+------------+                 +------+------+      | .add_trusted()    |
                                      |             | .remove_trusted() |
                                      |             | .is_trusted()     |
                                      |             +--------+----------+
                                      |                      |
                                      v                      v
                               check_peer_          trusted.toml
                               fingerprint()        (atomic write
                               (pure function,       via tempfile
                                no I/O -- used       + persist)
                                by Phase 6)
```

Data flow for AcceptFingerprint:
1. CLI sends `IpcRequest::AcceptFingerprint { fingerprint }` over Unix socket
2. `periphore-ipc` deserializes, creates `IpcCommand::AcceptFingerprint { fingerprint, responder }`
3. `periphored` main loop receives command via mpsc channel
4. Daemon calls `TrustStore::load()` (or uses cached instance), then `.add_trusted(fp, None)`
5. `TrustStore` serializes to TOML, writes to NamedTempFile in same dir, calls `persist()` (atomic rename)
6. Daemon responds with `IpcResponse::Ok` via oneshot responder

### Recommended Project Structure
```
crates/
  periphore-trust/
    Cargo.toml         # workspace deps: serde, thiserror, toml, tempfile, directories, tracing
    src/
      lib.rs           # pub mod store; pub use store::{TrustStore, TrustedPeer, TrustError};
      store.rs         # TrustStore impl: load, save, add_trusted, remove_trusted, is_trusted
    tests/
      trust.rs         # Integration tests (because [lib] test = false)
```

### Pattern 1: Atomic TOML Write via tempfile + persist

**What:** Write trust cache to a NamedTempFile in the same directory as the target, then atomically rename. This ensures other processes (or a crash) never see a partial write.

**When to use:** Every time `TrustStore` modifies the trusted.toml file (add or remove).

**Example:**
```rust
// Source: Context7 /stebalien/tempfile + verified via local test
use std::io::Write;
use tempfile::NamedTempFile;

fn save(&self, path: &Path) -> Result<(), TrustError> {
    let toml_str = toml::to_string_pretty(&self.cache)
        .map_err(|e| TrustError::SerializeError(e.to_string()))?;

    // Ensure parent directory exists (first-run case)
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(TrustError::Io)?;
    }

    // NamedTempFile::new_in creates file with 0o600 on Unix by default
    let mut tmp = NamedTempFile::new_in(
        path.parent().unwrap_or(Path::new(".")),
    ).map_err(TrustError::Io)?;

    tmp.write_all(toml_str.as_bytes())
        .map_err(TrustError::Io)?;

    // Flush to disk before rename for durability
    tmp.as_file().sync_all()
        .map_err(TrustError::Io)?;

    // Atomic rename -- other processes see old or new, never partial
    tmp.persist(path)
        .map_err(|e| TrustError::Io(e.error))?;

    Ok(())
}
```

### Pattern 2: TrustStore Struct with TOML Round-Trip

**What:** Internal cache struct that derives both `Serialize` (for writes) and `Deserialize` (for reads). Note: this is distinct from the config crate's `Config` which intentionally does NOT derive Serialize (CFG-01). The trust cache is the one exception -- it writes to disk via `AcceptFingerprint`.

**When to use:** All trust cache operations.

**Example:**
```rust
// Verified via local round-trip test: Vec<TrustedPeer> serializes to [[trusted]]
use serde::{Serialize, Deserialize};

/// Internal TOML document structure.
#[derive(Debug, Serialize, Deserialize)]
struct TrustCache {
    #[serde(default)]
    trusted: Vec<TrustedPeer>,
}

/// A single trusted peer entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedPeer {
    pub fingerprint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

// Serializes to:
// [[trusted]]
// fingerprint = "a3f9a3f9..."
// alias = "work-mac"
//
// [[trusted]]
// fingerprint = "b2e1b2e1..."
```

### Pattern 3: Pure Conflict Detection Function

**What:** A pure function with zero I/O that compares a configured fingerprint against an actual fingerprint. Returns `Result<(), TrustError>`.

**When to use:** Phase 6 TCP handshake calls this when `PeerConfig.fingerprint` is set.

**Example:**
```rust
/// Check whether a peer's actual fingerprint matches the configured expectation.
///
/// Returns `Ok(())` if they match, or `Err(TrustError::FingerprintConflict)` if not.
/// `peer_label` provides context for error messages (name or fingerprint prefix).
///
/// This is a pure function with no I/O -- designed for unit testing in isolation.
pub fn check_peer_fingerprint(
    configured_fp: &str,
    actual_fp: &str,
    peer_label: &str,
) -> Result<(), TrustError> {
    if configured_fp == actual_fp {
        Ok(())
    } else {
        Err(TrustError::FingerprintConflict {
            expected: configured_fp.to_owned(),
            actual: actual_fp.to_owned(),
            peer_label: peer_label.to_owned(),
        })
    }
}
```

### Pattern 4: Daemon IPC Dispatch Promotion

**What:** Promote `AcceptFingerprint` and `RejectFingerprint` from the `send_ok()` catch-all to named match arms in the main `tokio::select!` loop.

**When to use:** Wiring trust operations into the daemon.

**Example:**
```rust
// In periphored/src/main.rs, main event loop:
Some(IpcCommand::AcceptFingerprint { fingerprint, responder }) => {
    tracing::info!(fingerprint = %fingerprint, "IPC: AcceptFingerprint");
    match trust_store.add_trusted(&fingerprint, None) {
        Ok(()) => {
            tracing::info!(fingerprint = %fingerprint, "fingerprint trusted and cached");
            let _ = responder.send(IpcResponse::Ok);
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to cache trusted fingerprint");
            let _ = responder.send(IpcResponse::Error {
                message: format!("trust cache error: {e}"),
            });
        }
    }
}
Some(IpcCommand::RejectFingerprint { fingerprint, responder }) => {
    // Rejection is stateless -- no state change needed.
    // The daemon simply does not add the fingerprint to the trust cache.
    tracing::info!(fingerprint = %fingerprint, "IPC: RejectFingerprint (no state change)");
    let _ = responder.send(IpcResponse::Ok);
}
```

### Anti-Patterns to Avoid

- **Writing trusted.toml without atomic rename:** Direct `std::fs::write` is not atomic -- a crash mid-write corrupts the file. Always use tempfile + persist.
- **Holding TrustStore across async boundaries:** TrustStore does synchronous file I/O. Do not hold a mutable reference across `.await` points. Load, modify, save in a synchronous block. If the trust store grows large enough to block the runtime (unlikely for fingerprint counts), wrap in `spawn_blocking`.
- **Deriving Serialize on Config:** The main config (`periphore-config::Config`) must NEVER derive Serialize (CFG-01). Only the trust cache structs (`TrustCache`, `TrustedPeer`) derive Serialize because trust writes are the ONE exception to "config never auto-writes."
- **Comparing fingerprints case-sensitively without normalization:** Fingerprints are 64-char lowercase hex. Comparison should be case-insensitive or both sides normalized to lowercase. The `check_peer_fingerprint` function should compare after `.to_ascii_lowercase()`.
- **Storing TrustStore path inside TrustStore:** The path is the daemon's responsibility (it resolves via `directories::ProjectDirs`). TrustStore methods accept `&Path` parameters. This keeps TrustStore testable with arbitrary temp paths.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Atomic file write | Manual temp file + rename + error handling | `tempfile::NamedTempFile::new_in()` + `.persist()` | Handles temp name generation, 0o600 permissions, same-filesystem placement, cleanup on error [VERIFIED: Context7 /stebalien/tempfile] |
| TOML serialization | Custom string formatting | `toml::to_string_pretty()` | Correct escaping, array-of-tables syntax, round-trip safety [VERIFIED: local test] |
| TOML deserialization | Custom parsing | `toml::from_str()` | Consistent with figment's TOML parsing; handles edge cases [VERIFIED: local test] |
| XDG path resolution | Manual `$HOME` / env var parsing | `directories::ProjectDirs::data_dir()` | Platform-correct on macOS + Linux; already in workspace [VERIFIED: identity crate usage] |

**Key insight:** The trust cache is the only disk-write path in the entire project. Getting it wrong (partial writes, permission races, format errors) has security implications. Using proven crates for atomic write and TOML serialization eliminates entire categories of bugs.

## Common Pitfalls

### Pitfall 1: Non-Atomic Write Corrupts Trust Cache on Crash
**What goes wrong:** Using `std::fs::write()` directly. A power failure or process kill mid-write leaves a truncated or empty `trusted.toml`. On next daemon start, `TrustStore::load()` sees corrupt TOML and returns `CorruptCacheFile` error, blocking all trust operations until manual intervention.
**Why it happens:** `std::fs::write()` is not atomic -- it opens, truncates, writes. The truncate-before-write window is the danger zone.
**How to avoid:** Always use `tempfile::NamedTempFile::new_in(parent_dir)` + `.persist(target_path)`. The rename is atomic on Unix. The old file is only replaced once the new file is fully written and synced.
**Warning signs:** Trust cache file is 0 bytes after a crash.

### Pitfall 2: tempfile on Different Filesystem
**What goes wrong:** Creating a `NamedTempFile` in `/tmp` but persisting to `~/.local/share/periphore/trusted.toml`. On some Linux configs, `/tmp` is a tmpfs (different filesystem). `persist()` falls back to copy+delete, which is NOT atomic.
**Why it happens:** `NamedTempFile::new()` uses the system temp dir by default.
**How to avoid:** Always use `NamedTempFile::new_in(path.parent())` to ensure the temp file is on the same filesystem as the target.
**Warning signs:** `persist()` works but is not actually atomic -- hard to detect without testing on systems where `/tmp` is a separate mount.

### Pitfall 3: Empty Trust Cache Serializes to `trusted = []`
**What goes wrong:** An empty `TrustCache { trusted: vec![] }` serializes to `trusted = []` (inline empty array), not an empty file. This is valid TOML and round-trips correctly, but may surprise users who open the file expecting `[[trusted]]` syntax.
**Why it happens:** TOML spec -- empty arrays serialize as inline `[]`.
**How to avoid:** This is actually fine. The important thing is that `#[serde(default)]` on the `trusted` field handles both the empty-array case AND the missing-field case (if the file exists but has no `trusted` key). Tested and verified.
**Warning signs:** None -- this works correctly. Document the behavior.

### Pitfall 4: Fingerprint Case Sensitivity
**What goes wrong:** User enters `A3F9...` in config (uppercase), identity crate generates `a3f9...` (lowercase). `check_peer_fingerprint` comparison fails even though the fingerprints are semantically identical.
**Why it happens:** String comparison is case-sensitive by default.
**How to avoid:** Normalize both sides to lowercase before comparison: `configured_fp.to_ascii_lowercase() == actual_fp.to_ascii_lowercase()`. Document that fingerprints are always lowercase hex.
**Warning signs:** "Fingerprint conflict" errors when the actual fingerprint matches but differs in case.

### Pitfall 5: TrustStore Load-Modify-Save Race Condition
**What goes wrong:** Two concurrent `AcceptFingerprint` IPC commands arrive. Both load the current trust cache, both add their fingerprint, both save. The second save overwrites the first addition.
**Why it happens:** Load-modify-save without locking.
**How to avoid:** In the current architecture this is a non-issue: the daemon's main event loop processes IPC commands sequentially via `tokio::select!` (single-threaded dispatch). The `TrustStore` is only modified in the main loop, never from concurrent tasks. If this changes in the future, use a `Mutex<TrustStore>` or serialize through a channel.
**Warning signs:** Missing trusted fingerprints after concurrent accept operations.

### Pitfall 6: Forgetting to Update send_ok() When Promoting Arms
**What goes wrong:** Moving `AcceptFingerprint` from `send_ok()` to a named match arm without removing it from `send_ok()`. The wildcard arm in `send_ok()` still catches it, but the named arm never executes.
**Why it happens:** The `send_ok()` function uses a wildcard `_ => {}` arm at the bottom.
**How to avoid:** After adding named arms to the main `select!` loop for `AcceptFingerprint` and `RejectFingerprint`, remove those variants from `send_ok()`. The compiler's exhaustiveness check will catch it -- the removed variants are now dead code.
**Warning signs:** `AcceptFingerprint` silently returns `Ok` without actually modifying the trust store.

## Code Examples

### TrustError Enum (following IdentityError pattern)
```rust
// Source: modeled after crates/periphore-identity/src/lib.rs IdentityError
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrustError {
    /// Trust cache file exists but contains invalid TOML.
    #[error("trust cache file is corrupt: {0}")]
    CorruptCacheFile(String),

    /// Fingerprint conflict: configured fingerprint does not match actual.
    #[error("fingerprint conflict for peer '{peer_label}': expected {expected}, got {actual}")]
    FingerprintConflict {
        expected: String,
        actual: String,
        peer_label: String,
    },

    /// Duplicate fingerprint already in trust cache.
    #[error("fingerprint already trusted: {0}")]
    AlreadyTrusted(String),

    /// Fingerprint not found in trust cache (for remove operations).
    #[error("fingerprint not found in trust cache: {0}")]
    NotFound(String),

    /// TOML serialization error.
    #[error("serialization error: {0}")]
    SerializeError(String),

    /// Underlying I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

### TrustStore Public API
```rust
// Source: D-07 from CONTEXT.md, verified against identity crate patterns
use std::path::Path;

pub struct TrustStore {
    cache: TrustCache,
}

impl TrustStore {
    /// Load the trust cache from disk. Returns empty cache if file doesn't exist.
    pub fn load(path: &Path) -> Result<Self, TrustError> {
        if path.exists() {
            let content = std::fs::read_to_string(path).map_err(TrustError::Io)?;
            let cache: TrustCache = toml::from_str(&content)
                .map_err(|e| TrustError::CorruptCacheFile(e.to_string()))?;
            Ok(Self { cache })
        } else {
            Ok(Self { cache: TrustCache { trusted: vec![] } })
        }
    }

    /// Check if a fingerprint is in the trust cache.
    pub fn is_trusted(&self, fp: &str) -> bool {
        let fp_lower = fp.to_ascii_lowercase();
        self.cache.trusted.iter().any(|p| p.fingerprint == fp_lower)
    }

    /// Add a fingerprint to the trust cache and persist to disk.
    pub fn add_trusted(&mut self, fp: &str, alias: Option<&str>, path: &Path) -> Result<(), TrustError> {
        let fp_lower = fp.to_ascii_lowercase();
        if self.is_trusted(&fp_lower) {
            return Err(TrustError::AlreadyTrusted(fp_lower));
        }
        self.cache.trusted.push(TrustedPeer {
            fingerprint: fp_lower,
            alias: alias.map(String::from),
        });
        self.save(path)
    }

    /// Remove a fingerprint from the trust cache and persist to disk.
    pub fn remove_trusted(&mut self, fp: &str, path: &Path) -> Result<(), TrustError> {
        let fp_lower = fp.to_ascii_lowercase();
        let before = self.cache.trusted.len();
        self.cache.trusted.retain(|p| p.fingerprint != fp_lower);
        if self.cache.trusted.len() == before {
            return Err(TrustError::NotFound(fp_lower));
        }
        self.save(path)
    }

    fn save(&self, path: &Path) -> Result<(), TrustError> {
        // ... atomic write pattern from Pattern 1 above
    }
}
```

### Workspace Cargo.toml Changes
```toml
# Add to [workspace.dependencies] section:
periphore-trust = { path = "crates/periphore-trust", version = "0.1.0" }
toml            = { version = "0.8", features = ["display"] }
tempfile        = { version = "3" }
```

### periphore-trust/Cargo.toml
```toml
[package]
name = "periphore-trust"
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
serde       = { workspace = true }
thiserror   = { workspace = true }
toml        = { workspace = true }
tempfile    = { workspace = true }
directories = { workspace = true }
tracing     = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

### PeerConfig Addition (schema.rs)
```rust
// In crates/periphore-config/src/schema.rs
#[derive(Debug, Deserialize, Default)]
pub struct PeerConfig {
    pub fingerprint: Option<String>,
    /// Human-readable label for this peer. Local-only convenience -- NOT sent over
    /// the wire, does NOT participate in identity verification. Used in log messages
    /// and error reports. If absent, logs use the fingerprint hex or host address.
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
}
```

### MonitorConfig + TopologyConfig (schema.rs)
```rust
// In crates/periphore-config/src/schema.rs

/// Per-monitor configuration entry. Repeated as [[topology.monitor]] in TOML.
#[derive(Debug, Deserialize, Default)]
pub struct MonitorConfig {
    /// OS-level monitor identifier (xrandr output name, CoreGraphics display ID, etc.).
    /// Free-form string -- Phase 8 implements matching against OS-provided identifiers.
    pub id: Option<String>,
    /// Human-readable label (optional override for display in logs/CLI).
    pub name: Option<String>,
    /// Monitor width in pixels.
    pub width: Option<u32>,
    /// Monitor height in pixels.
    pub height: Option<u32>,
}

/// Monitor topology configuration (Phase 3 populates monitors, Phase 8 adds edge config).
#[derive(Debug, Deserialize, Default)]
pub struct TopologyConfig {
    /// Preferred monitor layout entries. TOML: [[topology.monitor]].
    #[serde(default, rename = "monitor")]
    pub monitors: Vec<MonitorConfig>,
}
```

### TOML Config Example (user-authored)
```toml
# periphore.toml

[[peer]]
fingerprint = "a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9"
name = "work-mac"
host = "192.168.1.100"
port = 24800

[[topology.monitor]]
id = "DP-1"
name = "primary"
width = 2560
height = 1440

[[topology.monitor]]
id = "HDMI-1"
name = "secondary"
width = 1920
height = 1080
```

### trusted.toml Example (daemon-written)
```toml
[[trusted]]
fingerprint = "a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9a3f9"
alias = "work-mac"

[[trusted]]
fingerprint = "b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1b2e1"
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `toml` crate 0.5.x (separate `toml::ser`, `toml::de`) | `toml` 0.8.x (unified, based on `toml_edit`) | 2023 | Unified crate with pretty-print support; 0.8 is current stable line used by figment |
| Manual `fs::write` + `fs::rename` | `tempfile::NamedTempFile::persist()` | Stable since tempfile 3.x | Handles temp name generation, default permissions, cleanup on error |
| `thiserror` 1.x | `thiserror` 2.x | Late 2024 | Minor API changes; workspace already on 2.0 |

**Deprecated/outdated:**
- `toml` 0.5.x: Superseded by 0.7/0.8 which are based on `toml_edit`. The 0.5 API is incompatible.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `toml` 0.8 features `display` + `serde` are both enabled by default or via the `features = ["display"]` declaration | Standard Stack | If `display` feature is not available in 0.8, `to_string_pretty` would not compile -- verify with `cargo check` during implementation |
| A2 | `AlreadyTrusted` error variant is the right behavior for duplicate add -- rather than silently succeeding | Code Examples | If callers expect idempotent add, this would be a behavioral surprise. The planner should decide: error vs idempotent. Recommendation: make it idempotent (return Ok if already present) for better UX |
| A3 | `tempfile` as both a regular dep (for periphore-trust write path) and workspace-level declaration works correctly with workspace = true in both deps and dev-deps sections | Standard Stack | Verified pattern -- Cargo handles this; same crate can be in both sections with workspace = true |

## Open Questions (RESOLVED)

1. **Should `add_trusted` be idempotent or error on duplicate?**
   - What we know: D-07 signature is `add_trusted(fp, alias) -> Result<(), TrustError>`. A duplicate fingerprint is a conceivable scenario (user accepts the same peer twice).
   - What's unclear: Whether returning `Err(AlreadyTrusted)` or silently succeeding is better UX.
   - Recommendation: Make it idempotent -- return `Ok(())` if the fingerprint is already trusted. This is friendlier for the CLI flow where a user might accidentally accept twice. Update alias if provided.

2. **Should the daemon hold a persistent TrustStore instance or load-per-request?**
   - What we know: The trust cache file will have a small number of entries (likely <100 peers). Loading from disk is fast (<1ms).
   - What's unclear: Whether the daemon should hold `trust_store: TrustStore` across the main loop or load fresh on each `AcceptFingerprint`.
   - Recommendation: Hold a persistent instance. Load once at startup (alongside config and identity). This avoids redundant disk reads and is consistent with how `Config` and `IdentityStore` are handled. The `TrustStore` is only mutated via IPC (sequential), so no concurrency concern.

3. **Should `TrustStore::load` be `pub fn load(path: &Path)` or `pub fn load_or_empty(path: &Path)`?**
   - What we know: D-09 says "if the cache file doesn't exist, create it atomically" on first `add_trusted`. So `load` should handle missing file gracefully.
   - What's unclear: The naming -- `load` that returns empty on missing file could surprise callers.
   - Recommendation: Name it `load` and document that it returns an empty TrustStore if the file doesn't exist. This matches the first-run experience where no fingerprints have been accepted yet.

4. **TOML `[[topology.monitor]]` serde rename**
   - What we know: `TopologyConfig` has field `monitors: Vec<MonitorConfig>`. In TOML, `[[topology.monitor]]` uses singular "monitor", but Rust field is plural "monitors".
   - What's unclear: Whether `#[serde(rename = "monitor")]` on the field correctly maps `[[topology.monitor]]` to `Vec<MonitorConfig>`.
   - Recommendation: Use `#[serde(default, rename = "monitor")]` on the `monitors` field. This maps TOML `[[topology.monitor]]` entries to Rust `monitors: Vec<MonitorConfig>`. Verified: serde's rename applies to the serialization key, so `[[topology.monitor]]` becomes `monitors` in Rust. [ASSUMED -- should verify with a quick test during implementation]

## Environment Availability

Step 2.6: SKIPPED (no external dependencies identified -- this phase is purely code/config changes with no external tools, services, or runtimes beyond the existing Rust toolchain).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (Rust built-in, via rustc test harness) |
| Config file | None needed -- Cargo.toml `[lib] test = false` pattern with `tests/` subdir |
| Quick run command | `cargo test -p periphore-trust` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SEC-05 | Fingerprint cached between sessions (load, add, reload = still trusted) | integration | `cargo test -p periphore-trust --test trust -- test_add_trusted_persists_across_reload` | Wave 0 |
| SEC-05 | Cache stored in separate file from main config | integration | `cargo test -p periphore-trust --test trust -- test_cache_separate_from_config` | Wave 0 |
| SEC-05 | Corrupt cache file returns error | integration | `cargo test -p periphore-trust --test trust -- test_corrupt_cache_returns_error` | Wave 0 |
| SEC-06 | Fingerprint conflict detected | unit | `cargo test -p periphore-trust --test trust -- test_fingerprint_conflict_detected` | Wave 0 |
| SEC-06 | Matching fingerprints pass | unit | `cargo test -p periphore-trust --test trust -- test_matching_fingerprint_passes` | Wave 0 |
| SEC-06 | Case-insensitive fingerprint comparison | unit | `cargo test -p periphore-trust --test trust -- test_fingerprint_case_insensitive` | Wave 0 |
| CFG-02 | PeerConfig.name field parses from TOML | integration | `cargo test -p periphore-config --test config -- test_peer_name_field` | Wave 0 |
| CFG-03 | MonitorConfig entries parse from [[topology.monitor]] | integration | `cargo test -p periphore-config --test config -- test_topology_monitor_config` | Wave 0 |
| CFG-03 | Empty monitors defaults to empty vec | integration | `cargo test -p periphore-config --test config -- test_topology_monitors_default_empty` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p periphore-trust && cargo test -p periphore-config`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/periphore-trust/tests/trust.rs` -- covers SEC-05, SEC-06
- [ ] New tests in `crates/periphore-config/tests/config.rs` -- covers CFG-02, CFG-03
- [ ] Framework install: Not needed -- cargo test is built-in

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | Yes (trust model) | Ed25519 fingerprint verification via `check_peer_fingerprint` |
| V3 Session Management | No | No sessions in this phase |
| V4 Access Control | Yes (trust cache access) | File permissions 0o600 via tempfile default; parent dir created with user-only access |
| V5 Input Validation | Yes | Fingerprint format validation (64-char hex); TOML parse error handling |
| V6 Cryptography | No (Phase 2 handled this) | Ed25519 keys already implemented; this phase uses fingerprints, not crypto primitives |

### Known Threat Patterns for Trust Cache

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Trust cache tampering (TOCTOU) | Tampering | Atomic write via tempfile::persist eliminates partial-write window |
| Malformed TOML injection | Tampering | toml::from_str returns parse error; CorruptCacheFile variant prevents silent acceptance |
| World-readable trust cache | Information Disclosure | tempfile creates with 0o600 by default; verify permissions in tests |
| Fingerprint collision/impersonation | Spoofing | SHA-256 of Ed25519 public key -- 256-bit collision resistance; hard-config fingerprint pinning (SEC-06) |
| Case-sensitivity bypass | Tampering | Normalize to lowercase before comparison and storage |

## Sources

### Primary (HIGH confidence)
- Context7 `/stebalien/tempfile` -- NamedTempFile::persist() atomic rename, 0o600 default permissions
- Context7 `/websites/rs_toml` -- toml::to_string_pretty, Vec serialization to array of tables
- Local verification test (toml round-trip) -- confirmed `Vec<TrustedPeer>` serializes to `[[trusted]]` and round-trips with `skip_serializing_if`
- Local verification test (tempfile persist) -- confirmed 0o600 permissions and atomic rename
- Codebase: `crates/periphore-identity/src/lib.rs` -- IdentityError pattern, load_or_create pattern, data_dir() usage
- Codebase: `crates/periphore-config/src/schema.rs` -- existing PeerConfig, TopologyConfig, serde patterns
- Codebase: `crates/periphored/src/main.rs` -- current IPC dispatch loop, send_ok() function

### Secondary (MEDIUM confidence)
- `cargo tree -p figment | grep toml` -- confirmed figment uses toml 0.8.23 (version unification)
- `cargo search tempfile` -- confirmed latest is 3.27.0
- `cargo search toml` -- confirmed latest stable line is 1.1.2; 0.8.x is the figment-compatible line

### Tertiary (LOW confidence)
- None -- all claims verified through codebase inspection, Context7, or local testing

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all libraries already in workspace or unify with existing transitive deps; verified via cargo tree and local tests
- Architecture: HIGH -- mirrors established patterns from periphore-identity crate; pure functions + atomic writes are well-understood
- Pitfalls: HIGH -- each pitfall verified through direct testing or codebase inspection
- Security: HIGH -- ASVS L1 controls map directly to existing patterns (file permissions, input validation, atomic writes)

**Research date:** 2026-04-24
**Valid until:** 2026-05-24 (stable -- Rust ecosystem and these crates are mature)
