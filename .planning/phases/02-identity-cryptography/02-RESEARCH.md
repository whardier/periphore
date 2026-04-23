# Phase 2: Identity & Cryptography — Research

**Researched:** 2026-04-22
**Domain:** Rust cryptographic identity — Ed25519 keypairs, SHA-256 fingerprints, OpenSSH Drunken Bishop identicons, BIP39 word phrases
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Raw 32-byte Ed25519 seed stored as a plain binary file.
- **D-02:** Storage path: `{XDG_DATA_HOME}/periphore/key` — Linux: `~/.local/share/periphore/key`, macOS: `~/Library/Application Support/periphore/key`. Use `directories::ProjectDirs`.
- **D-03:** File permissions `0600` set immediately after creation on Unix.
- **D-04:** Parent directory created on first run by `IdentityStore` if it does not exist.
- **D-05:** Drunken Bishop algorithm, 17 columns x 9 rows, start at center (8, 4).
- **D-06:** Header `+--[ED25519 256]--+`, footer `+--[PERIPHORE]----+`.
- **D-07:** Input to randomart: the 32-byte SHA-256 fingerprint of the public key.
- **D-08:** Identicon disable via `identity.show_identicon = false` config flag.
- **D-09:** `GetIdenticon` IPC response: `{ fingerprint_hex: String, identicon: String }`.
- **D-10:** `GetWordPhrase` IPC response: `{ words: Vec<String>, phrase: String }`.
- **D-11:** BIP39 standard 2048-word list inlined as `static BIP39_WORDS: &[&str; 2048]` — no external crate.
- **D-12:** 6 words derived from SHA-256 fingerprint, 6 x 11-bit index windows.
- **D-13:** Space-delimited lowercase, no punctuation.
- **D-14:** `IdentityStore` struct with `load_or_create(path)`, `fingerprint_hex()`, `identicon()`, `word_phrase()`.
- **D-15:** First-run auto-generate with `tracing::info!("Generated new identity: {}", fingerprint_hex)`.
- **D-16:** Subsequent runs: reconstruct from 32-byte seed. Wrong length => `IdentityError::CorruptKeyFile`.
- **D-17:** `thiserror`-derived `IdentityError` in identity crate; `anyhow` only at daemon boundary.

### Claude's Discretion

- Exact Drunken Bishop character set — use `" .o+=*BOX@%&#/^SE"` exactly (OpenSSH standard).
- Whether to use `rand::rngs::OsRng` directly or `getrandom` for CSPRNG seed generation.
- Whether `IdentityStore::identicon()` accepts an optional width override parameter.
- BIP39 index derivation: exact bit-slicing approach (6 x 11-bit windows from 32-byte hash).

### Deferred Ideas (OUT OF SCOPE)

- Fingerprint caching and trust acceptance (SEC-05, SEC-06) — Phase 3
- TCP handshake using identity for peer authentication — Phase 6
- `periphore identity` CLI subcommand — Phase 5
- Key rotation or re-generation command — post-v1
- Identity backup/export command — post-v1
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SEC-01 | Each node generates a persistent Ed25519 keypair; fingerprint derived from public key | `SigningKey::generate` (rand_core feature), `SigningKey::from_bytes` for load, SHA-256 via `sha2::Sha256::digest` |
| SEC-02 | Fingerprint displayed as identicon (visual, shown on both machines simultaneously) | Drunken Bishop algorithm fully specified; pre-rendered to `String` in identity crate, returned via `GetIdenticon` IPC response |
| SEC-03 | Fingerprint available as typed word phrase (one side reads, other types) | BIP39 2048-word inlined list, 6 x 11-bit extraction from 32-byte fingerprint, returned via `GetWordPhrase` IPC response |
| SEC-04 | Identicon display can be disabled for headless/automated setups | `identity.show_identicon: bool` added to `IdentityConfig` in `periphore-config`; checked by CLI before printing identicon field |
</phase_requirements>

---

## Summary

Phase 2 implements the `periphore-identity` crate from a two-line stub into a fully functional cryptographic identity substrate. The work divides naturally into four independent capabilities: (1) keypair lifecycle (generate, persist, load), (2) fingerprint derivation, (3) identicon rendering, and (4) word phrase derivation. A fifth task integrates the identity into `periphored` and `periphore-protocol`.

The critical dependency question for this phase is rand/CSPRNG integration with ed25519-dalek 2.2.0. Ed25519-dalek 2.2.0 uses `rand_core ^0.6.4` (optional, gated by the `rand_core` feature). `rand_core 0.6.x` provides `OsRng` directly when the `getrandom` feature is enabled. The workspace must NOT use `rand 0.9.x` — that depends on `rand_core ^0.9.0` which is incompatible with ed25519-dalek 2.2.0's `rand_core` feature gate. The correct approach is to add `rand_core = { version = "0.6", features = ["getrandom"] }` to the workspace and use `rand_core::OsRng` directly. Alternatively, `rand 0.8.6` (the latest 0.8.x) also uses `rand_core ^0.6.0` and is compatible.

All three output formats (hex fingerprint, identicon, word phrase) are deterministic pure functions of the SHA-256 hash of the public key bytes — this is the strongest correctness property of the phase and must be validated via golden-value tests with a known seed.

**Primary recommendation:** Use `rand_core 0.6` with `features = ["getrandom"]` directly in the identity crate (no `rand` needed). Use `SigningKey::generate(&mut OsRng)` requiring the `rand_core` feature on `ed25519-dalek`. Implement Drunken Bishop and BIP39 extraction as pure functions on `[u8; 32]` — no external crates for either.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Keypair generation and persistence | `periphore-identity` | — | Identity crate owns all I/O for its own data (D-14) |
| Fingerprint derivation (SHA-256) | `periphore-identity` | — | Pure crypto, no external I/O |
| Identicon rendering | `periphore-identity` | — | Pre-rendered string; CLI prints verbatim (D-09) |
| Word phrase derivation | `periphore-identity` | — | Pure derivation from fingerprint bytes |
| IPC response structs | `periphore-protocol` | — | Response types added here for GetIdenticon/GetWordPhrase |
| Config flag `show_identicon` | `periphore-config` | — | New `IdentityConfig` section in schema |
| IPC dispatch wiring | `periphored` (daemon) | — | Replace `send_ok` stubs with real IdentityStore calls |
| Fingerprint in GetStatus | `periphored` (daemon) | — | Fill in `fingerprint: Some(...)` in GetStatus response |

---

## Implementation Strategy

### 1. Dependency Wiring

**ed25519-dalek 2.2.0 `rand_core` feature:** The `rand_core` feature on `ed25519-dalek` is optional and unlocks `SigningKey::generate<R: CryptoRngCore>(&mut csprng)`. [VERIFIED: ed25519-dalek 2.2.0 source at `~/.cargo/registry/src/.../ed25519-dalek-2.2.0/src/signing.rs`]

**CSPRNG choice — rand_core 0.6 vs rand 0.8:**
- `rand_core 0.6.4` (already transitively present in workspace) provides `OsRng` when `getrandom` feature is enabled. This is the minimal approach — no extra crate needed beyond enabling the feature. [VERIFIED: rand_core 0.6.4 source at `~/.cargo/registry/.../rand_core-0.6.4/src/os.rs`]
- `rand 0.8.6` (latest 0.8.x, released 2026-04-17) also uses `rand_core ^0.6.0` and is fully compatible. Adds `~5 transitive deps` vs the direct rand_core approach. [VERIFIED: crates.io API]
- `rand 0.9.x` — DO NOT USE. It requires `rand_core ^0.9.0` which is incompatible with ed25519-dalek 2.2.0's `rand_core` feature gate. [VERIFIED: crates.io dependency check]

**Recommendation:** Add `rand_core = { version = "0.6", features = ["getrandom"] }` to `[workspace.dependencies]`. Enable `rand_core` feature on `ed25519-dalek` in the identity crate's Cargo.toml. Use `rand_core::OsRng` in identity crate only.

**Cargo.toml changes needed:**
1. Add `rand_core = { version = "0.6", features = ["getrandom"] }` to workspace `[workspace.dependencies]`
2. Update `ed25519-dalek` workspace dep to `{ version = "2.2", features = ["rand_core"] }` (or add features only in identity crate's dep declaration)
3. Add to `periphore-identity/Cargo.toml`:
   - `rand_core = { workspace = true }`
   - `thiserror = { workspace = true }`
   - `directories = { workspace = true }`
4. Feature-gate: use `#[cfg(feature = "rand_core")]` is handled automatically since the feature is always enabled for the identity crate

**Note on workspace feature merging:** Because `ed25519-dalek` is declared in `[workspace.dependencies]`, adding features in the identity crate's local dep declaration overrides workspace-level feature set for that crate. The cleanest approach is to add `features = ["rand_core"]` to the workspace-level `ed25519-dalek` entry since no other crate needs to opt out.

### 2. IdentityStore — Keypair Lifecycle

```rust
// Source: ed25519-dalek 2.2.0 signing.rs + std::os::unix::fs::PermissionsExt
use std::{fs, io, path::Path};
use std::os::unix::fs::PermissionsExt;
use ed25519_dalek::SigningKey;
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("key file is corrupt (expected 32 bytes, got {0})")]
    CorruptKeyFile(usize),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("no valid data directory found for this user")]
    NoDataDir,
}

pub struct IdentityStore {
    pub keypair: SigningKey,
    pub fingerprint: [u8; 32],
}

impl IdentityStore {
    pub fn load_or_create(path: &Path) -> Result<Self, IdentityError> {
        if path.exists() {
            // Load existing seed
            let bytes = fs::read(path)?;
            if bytes.len() != 32 {
                return Err(IdentityError::CorruptKeyFile(bytes.len()));
            }
            let seed: [u8; 32] = bytes.try_into().unwrap();
            let keypair = SigningKey::from_bytes(&seed);
            let fingerprint = Self::compute_fingerprint(&keypair);
            Ok(Self { keypair, fingerprint })
        } else {
            // Generate new keypair
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let keypair = SigningKey::generate(&mut OsRng);
            let seed = keypair.to_bytes();
            fs::write(path, seed)?;
            // Set 0600 permissions immediately
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
            let fingerprint = Self::compute_fingerprint(&keypair);
            Ok(Self { keypair, fingerprint })
        }
    }

    fn compute_fingerprint(keypair: &SigningKey) -> [u8; 32] {
        let pubkey_bytes = keypair.verifying_key().to_bytes();
        Sha256::digest(pubkey_bytes).into()
    }
}
```

[VERIFIED: `SigningKey::from_bytes(&[u8; 32])` — confirmed in signing.rs source]
[VERIFIED: `SigningKey::to_bytes()` returns `[u8; 32]` (the seed) — confirmed in signing.rs]
[VERIFIED: `SigningKey::generate<R: CryptoRngCore>(&mut csprng)` — confirmed in signing.rs, gated by `rand_core` feature]
[VERIFIED: `PermissionsExt::from_mode(0o600)` — stable std::os::unix API]

### 3. Fingerprint Hex

```rust
impl IdentityStore {
    pub fn fingerprint_hex(&self) -> String {
        self.fingerprint.iter().map(|b| format!("{b:02x}")).collect()
    }
}
```

Output: 64 lowercase hex characters. Deterministic. [ASSUMED — standard hex encoding, no verification needed beyond correctness]

### 4. Drunken Bishop Identicon

The OpenSSH Drunken Bishop algorithm (from OpenSSH `sshkey.c`):

**Character table:** `" .o+=*BOX@%&#/^SE"` — exactly 17 characters at indices 0–16. Start position marked `S` (index 15), end position marked `E` (index 16). [VERIFIED: matches CONTEXT.md D-05 and OpenSSH convention; verified Python simulation in research session]

**Grid:** 17 columns (x: 0–16), 9 rows (y: 0–8). Start: (8, 4). [VERIFIED: CONTEXT.md D-05]

**Algorithm:**
1. Initialize 153-cell count grid to zero.
2. Set `(col, row) = (8, 4)`.
3. For each byte in the 32-byte input fingerprint:
   - Process 4 steps of 2 bits each (LSB first within the byte):
     - `bits = byte & 0x3`; `byte >>= 2`
     - `dx = if bits & 0x01 != 0 { 1 } else { -1 }`
     - `dy = if bits & 0x02 != 0 { 1 } else { -1 }`
     - `col = (col + dx).clamp(0, 16)`
     - `row = (row + dy).clamp(0, 8)`
     - `grid[row * 17 + col] += 1`
4. Record `end_pos = row * 17 + col` (final position after all steps).
5. Mark `start_pos = 4 * 17 + 8 = 76` (center).
6. Build output:
   - Header: `format!("+--[{}]{}+", label, "-".repeat(17 - 4 - label.len()))`
   - 9 rows of `|` + 17 chars + `|`
   - For each cell: if `pos == end_pos` → `E`, else if `pos == start_pos` → `S`, else `CHARS[grid[pos].min(16)]`
   - Footer: `format!("+--[{}]{}+", footer, "-".repeat(17 - 4 - footer.len()))`

[VERIFIED: Python simulation in research session confirmed algorithm produces correct 19-char-wide lines and matches D-06 header/footer format]

**Header/footer format:** `'+--[' + label + ']' + '-'.repeat(17 - 2 - 1 - label.len() - 1) + '+'`
- `'ED25519 256'` (11 chars) → `'+--[ED25519 256]--+'` (2 right dashes) [VERIFIED: Python simulation]
- `'PERIPHORE'` (9 chars) → `'+--[PERIPHORE]----+'` (4 right dashes) [VERIFIED: Python simulation, matches D-06]

**Note on start/end overlap:** If the bishop's final position happens to equal the center (start), display `E` (end takes priority). If start position is never revisited, it displays `S`. [ASSUMED — follows OpenSSH convention]

### 5. BIP39 Word Phrase

**Index extraction algorithm:** Treat the 32-byte fingerprint as a big-endian bit stream (256 bits). Extract 6 sequential non-overlapping 11-bit windows starting from the MSB.

```rust
fn word_indices(fingerprint: &[u8; 32]) -> [usize; 6] {
    let mut indices = [0usize; 6];
    for i in 0..6 {
        let bit_offset = i * 11;
        let byte_offset = bit_offset / 8;
        let bit_shift = bit_offset % 8;
        // Read 3 bytes to safely span any 11-bit window
        let window = ((fingerprint[byte_offset] as u32) << 16
            | (fingerprint[byte_offset + 1] as u32) << 8
            | (fingerprint[byte_offset + 2] as u32))
            >> (13 - bit_shift);
        indices[i] = (window & 0x7FF) as usize;
    }
    indices
}
```

[VERIFIED: Python cross-validation shows this produces identical results to big-integer extraction across 5 random test inputs]

**BIP39 wordlist:** Inline as `static BIP39_WORDS: &[&str; 2048]` in `periphore-identity/src/bip39.rs`. Source: https://github.com/trezor/python-mnemonic/blob/master/src/mnemonic/wordlist/english.txt — 2048 words, one per line. [ASSUMED — well-known public standard; planner should include inlining as a Wave 0 task]

**Word phrase output:**
```rust
pub fn word_phrase(&self) -> Vec<String> {
    word_indices(&self.fingerprint)
        .iter()
        .map(|&i| BIP39_WORDS[i].to_owned())
        .collect()
}
```

### 6. Config: IdentityConfig

Add to `periphore-config/src/schema.rs`:

```rust
#[derive(Debug, Deserialize)]
pub struct IdentityConfig {
    /// Show identicon on startup and in IPC GetIdenticon responses.
    /// Set to false for headless or automated setups (SEC-04).
    pub show_identicon: bool,
}

impl Default for IdentityConfig {
    fn default() -> Self { Self { show_identicon: true } }
}
```

Add field to `Config`:

```rust
pub struct Config {
    // ... existing fields ...
    #[serde(default)]
    pub identity: IdentityConfig,
}
```

**Config env var compatibility:** `show_identicon` contains an underscore — see the constraint in `periphore-config/src/lib.rs`: field names with underscores break Figment's `Env::prefixed("PERIPHORE_").split("_")` mapping. However, `identity.show_identicon` would map to env var `PERIPHORE_IDENTITY_SHOW_IDENTICON` which would split as `identity.show.identicon` (wrong). This is fine because SEC-04 only requires a config file flag — no env var override is needed for this field. Document this in a comment. [VERIFIED: existing warning in periphore-config/src/lib.rs; the pattern is established]

### 7. IpcResponse: New Variants

Add to `periphore-protocol/src/ipc.rs`:

```rust
pub enum IpcResponse {
    // ... existing variants ...
    Identicon {
        fingerprint_hex: String,
        identicon: String,
    },
    WordPhrase {
        words: Vec<String>,
        phrase: String,
    },
}
```

[VERIFIED: current `IpcResponse` in `periphore-protocol/src/ipc.rs` has `Status`, `Peers`, `Ok`, `Error` — no Identicon or WordPhrase variants yet]

### 8. periphored Dispatch Wiring

Current state: `GetIdenticon` and `GetWordPhrase` arms in `send_ok()` return `IpcResponse::Ok` (stub). [VERIFIED: periphored/src/main.rs lines 202–207]

Phase 2 changes:
1. Load `IdentityStore` at daemon startup (after config load, before IPC loop).
2. Pass a reference or `Arc` to the identity into the select! loop.
3. Replace stubs in `send_ok()` with real calls:

```rust
IpcCommand::GetIdenticon { responder, .. } => {
    let _ = responder.send(IpcResponse::Identicon {
        fingerprint_hex: identity.fingerprint_hex(),
        identicon: identity.identicon(),
    });
}
IpcCommand::GetWordPhrase { responder, .. } => {
    let words = identity.word_phrase();
    let phrase = words.join(" ");
    let _ = responder.send(IpcResponse::WordPhrase { words, phrase });
}
```

4. Fill in `GetStatus` fingerprint field:

```rust
IpcCommand::GetStatus { responder } => {
    let _ = responder.send(IpcResponse::Status {
        running: true,
        fingerprint: Some(identity.fingerprint_hex()),
    });
}
```

**IPC dispatch architecture note:** The current `periphored` main loop does not pass state into `send_ok()`. Phase 2 needs to move `GetIdenticon` and `GetWordPhrase` out of `send_ok()` into the main `select!` arms (which have access to the `identity` binding), similar to how `GetStatus` already has its own arm. [VERIFIED: periphored/src/main.rs structure]

---

## API Details

### ed25519-dalek 2.2.0 [VERIFIED: signing.rs source]

| Method | Signature | Notes |
|--------|-----------|-------|
| `SigningKey::from_bytes` | `fn from_bytes(secret_key: &[u8; 32]) -> Self` | Reconstructs from 32-byte seed; no Result — always valid |
| `SigningKey::to_bytes` | `fn to_bytes(&self) -> [u8; 32]` | Returns the 32-byte seed |
| `SigningKey::generate` | `fn generate<R: CryptoRngCore>(csprng: &mut R) -> Self` | Requires `rand_core` feature |
| `SigningKey::verifying_key` | `fn verifying_key(&self) -> VerifyingKey` | Returns the public key |
| `VerifyingKey::to_bytes` | `fn to_bytes(&self) -> [u8; 32]` | Returns the 32 public key bytes for hashing |

### sha2 0.10.9 [VERIFIED: sha2 lib.rs source]

```rust
use sha2::{Digest, Sha256};
let hash: [u8; 32] = Sha256::digest(&pubkey_bytes).into();
```

One-shot API. `Sha256::digest` accepts `AsRef<[u8]>`. Returns `GenericArray<u8, U32>` which converts to `[u8; 32]` via `.into()`.

### rand_core 0.6.4 OsRng [VERIFIED: rand_core 0.6.4 os.rs source]

```rust
use rand_core::OsRng;
// OsRng is a zero-sized struct; instantiate directly:
let signing_key = SigningKey::generate(&mut OsRng);
```

`OsRng` implements `CryptoRng + RngCore`. The `fill_bytes` method calls `getrandom::getrandom()` which uses the OS entropy source. No initialization needed.

### directories 6.0.0 ProjectDirs [VERIFIED: directories-6.0.0 source + macOS/Linux platform impls]

```rust
use directories::ProjectDirs;

fn key_path() -> Option<std::path::PathBuf> {
    ProjectDirs::from("", "", "periphore")
        .map(|dirs| dirs.data_dir().join("key"))
}
```

| Platform | `data_dir()` result |
|----------|---------------------|
| Linux | `$XDG_DATA_HOME/periphore` or `~/.local/share/periphore` |
| macOS | `~/Library/Application Support/periphore` |

**Critical:** macOS `data_dir()` == `config_dir()` == `Library/Application Support/...` (confirmed in source). The project path for `ProjectDirs::from("", "", "periphore")` is just `"periphore"` (no qualifier/org prefix since both are empty strings and parts list retains only non-empty).

### std::os::unix::fs::PermissionsExt [ASSUMED — stable std API]

```rust
use std::os::unix::fs::PermissionsExt;
std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
```

Must be called after write completes. `from_mode` is Unix-only (compile error on Windows — acceptable since project targets macOS + Linux only).

---

## Dependency Analysis

### Workspace Cargo.toml Changes

| Change | Location | Action |
|--------|----------|--------|
| Add `rand_core` to workspace deps | `Cargo.toml [workspace.dependencies]` | `rand_core = { version = "0.6", features = ["getrandom"] }` |
| Enable `rand_core` feature on `ed25519-dalek` | `Cargo.toml [workspace.dependencies]` | Add `features = ["rand_core"]` to existing `ed25519-dalek = { version = "2.2" }` entry |

### periphore-identity/Cargo.toml Changes

| Addition | Workspace ref | Purpose |
|----------|---------------|---------|
| `rand_core = { workspace = true }` | Yes | `OsRng` for keypair generation |
| `thiserror = { workspace = true }` | Yes | `IdentityError` derive |
| `directories = { workspace = true }` | Yes | `ProjectDirs` for key path resolution |

**No new external crates needed.** All dependencies are already in `[workspace.dependencies]` except `rand_core`, which is a transitive dep being promoted to direct.

### periphored/Cargo.toml Changes

Add `periphore-identity = { workspace = true }` as a direct dependency (it is currently a stub with no imports; Phase 2 makes it real).

### Version Compatibility Matrix [VERIFIED: crates.io API]

| Crate | Version | rand_core compat | Status |
|-------|---------|-----------------|--------|
| ed25519-dalek | 2.2.0 | `^0.6.4` (optional) | In workspace — enable `rand_core` feature |
| rand_core | 0.6.4 | N/A (is rand_core) | Add to workspace deps with `getrandom` feature |
| rand | 0.8.6 | `^0.6.0` | NOT needed; direct rand_core is cleaner |
| rand | 0.9.x | `^0.9.0` | INCOMPATIBLE — do not use |
| getrandom | 0.4.2 | N/A (pulled by rand_core) | Already in workspace transitively |

---

## Integration Points

### File Changes Summary

| File | Change |
|------|--------|
| `Cargo.toml` | Add `rand_core` to workspace deps; add `rand_core` feature to `ed25519-dalek` |
| `crates/periphore-identity/Cargo.toml` | Add `rand_core`, `thiserror`, `directories` deps |
| `crates/periphore-identity/src/lib.rs` | Implement `IdentityStore`, `IdentityError`, `identicon()`, `word_phrase()` |
| `crates/periphore-identity/src/bip39.rs` | Inline BIP39 wordlist as `static BIP39_WORDS` |
| `crates/periphore-identity/tests/identity.rs` | Integration tests (golden values, error paths, persistence) |
| `crates/periphore-protocol/src/ipc.rs` | Add `Identicon` and `WordPhrase` variants to `IpcResponse` |
| `crates/periphore-config/src/schema.rs` | Add `IdentityConfig` struct and `identity` field to `Config` |
| `crates/periphored/Cargo.toml` | Add `periphore-identity = { workspace = true }` |
| `crates/periphored/src/main.rs` | Load identity at startup; dispatch `GetIdenticon`, `GetWordPhrase`, fill `GetStatus` fingerprint |

### periphored main.rs Integration Pattern

The `IdentityStore` must be initialized before the `select!` loop. The IPC dispatch arms for `GetIdenticon` and `GetWordPhrase` must be moved out of `send_ok()` (which receives ownership of the command but has no access to state) into the main `select!` loop. The current `send_ok()` function is exhaustiveness-enforcing for stateless commands — `GetIdenticon` and `GetWordPhrase` become stateful and must exit that function.

**Startup pattern:**
```rust
// After config load, before spawning tasks:
let key_path = periphore_identity::default_key_path()
    .ok_or_else(|| anyhow::anyhow!("cannot determine key storage path"))?;
let identity = periphore_identity::IdentityStore::load_or_create(&key_path)
    .map_err(|e| anyhow::anyhow!("identity error: {e}"))?;
tracing::info!(
    fingerprint = %identity.fingerprint_hex(),
    "identity loaded"
);
```

The `default_key_path()` free function is a convenience helper in `periphore-identity` that calls `ProjectDirs` — keeps path resolution out of `periphored/main.rs`.

---

## Validation Architecture

### Test Framework [VERIFIED: existing workspace]

| Property | Value |
|----------|-------|
| Framework | Rust built-in tests via `cargo test` |
| Test location | `crates/periphore-identity/tests/identity.rs` (integration tests) — enforced by `[lib] test = false` in Cargo.toml |
| Quick run command | `cargo test -p periphore-identity` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SEC-01 | `load_or_create` generates keypair on first call, persists to file | integration | `cargo test -p periphore-identity -- test_first_run_creates_key_file` | Wave 0 |
| SEC-01 | `load_or_create` loads same keypair on second call | integration | `cargo test -p periphore-identity -- test_load_after_create_is_identical` | Wave 0 |
| SEC-01 | Fingerprint is deterministic: same key seed → same 64-char hex | unit | `cargo test -p periphore-identity -- test_fingerprint_determinism` | Wave 0 |
| SEC-01 | Corrupt key file (wrong length) → `IdentityError::CorruptKeyFile` | integration | `cargo test -p periphore-identity -- test_corrupt_key_file_error` | Wave 0 |
| SEC-02 | Identicon is deterministic: same fingerprint → identical 17×9 output | unit | `cargo test -p periphore-identity -- test_identicon_determinism` | Wave 0 |
| SEC-02 | Identicon header is `+--[ED25519 256]--+`, footer is `+--[PERIPHORE]----+` | unit | `cargo test -p periphore-identity -- test_identicon_borders` | Wave 0 |
| SEC-02 | Identicon output is exactly 11 lines (header + 9 rows + footer) | unit | `cargo test -p periphore-identity -- test_identicon_line_count` | Wave 0 |
| SEC-03 | Word phrase is deterministic: same fingerprint → same 6 words | unit | `cargo test -p periphore-identity -- test_word_phrase_determinism` | Wave 0 |
| SEC-03 | Word phrase has exactly 6 words, all lowercase, all valid BIP39 words | unit | `cargo test -p periphore-identity -- test_word_phrase_validity` | Wave 0 |
| SEC-04 | Config `show_identicon = false` parses and defaults to `true` | unit (config crate) | `cargo test -p periphore-config -- test_identity_config_defaults` | Wave 0 |

### Golden-Value Tests (Critical for Cross-Platform Determinism)

The most important tests for SEC-01 through SEC-03 are **golden-value tests** that verify a known input seed produces the exact expected output on all platforms.

Proposed golden values (computed during implementation from a deterministic test seed):

```rust
// Test seed: all zeros (deterministic, never use in production)
const TEST_SEED: [u8; 32] = [0u8; 32];
// Expected fingerprint_hex: derive during implementation and hard-code here
// Expected identicon: derive during implementation and hard-code (19 chars x 11 lines)
// Expected words: derive during implementation and hard-code
```

[VERIFIED: Python simulation confirms the algorithm is deterministic; exact golden values computed during Wave 1 implementation and hard-coded in Wave 2 tests]

### Sampling Rate

- **Per task commit:** `cargo test -p periphore-identity`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full workspace suite green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `crates/periphore-identity/tests/identity.rs` — all identity integration and unit tests (no test infrastructure exists yet in this crate)
- [ ] `crates/periphore-config/tests/config.rs` extension — add `identity` config section tests (file exists, add cases)

---

## Pitfalls & Landmines

### Pitfall 1: rand_core Version Conflict

**What goes wrong:** Adding `rand = "0.9"` or any crate that depends on `rand_core 0.9` will cause ed25519-dalek's `rand_core` feature (which gates on `rand_core 0.6`) to fail to compile or silently use the wrong type.

**Why it happens:** Cargo can unify minor versions but not major versions of the same crate. `rand_core 0.6` and `rand_core 0.9` are different crates in Cargo's resolution model.

**How to avoid:** Use `rand_core = { version = "0.6", features = ["getrandom"] }` directly. Do not add `rand` unless specifically needed. If `rand` is added later, use `rand 0.8.x` (not 0.9.x).

**Warning signs:** `error[E0277]: the trait bound rand_core::CryptoRng is not satisfied` — this is the canonical error when rand_core versions conflict.

### Pitfall 2: ed25519-dalek `generate` Not Available Without Feature

**What goes wrong:** `SigningKey::generate` is `#[cfg(any(test, feature = "rand_core"))]` — calling it without enabling the feature produces a compile error or silently falls back to an unstable API.

**Why it happens:** ed25519-dalek feature-gates the `rand_core` dependency as optional (confirmed in Cargo.toml features inspection).

**How to avoid:** Declare `ed25519-dalek = { workspace = true, features = ["rand_core"] }` in `periphore-identity/Cargo.toml` (or add `features = ["rand_core"]` to the workspace dep entry, which propagates automatically).

### Pitfall 3: Drunken Bishop Bit Order

**What goes wrong:** Implementing the algorithm with MSB-first bit extraction within each byte instead of LSB-first produces a different (wrong) identicon.

**Why it happens:** The algorithm processes bits as `byte & 0x3` then `byte >>= 2` — this takes LSBs first. A natural reading of "process 2 bits at a time" might lead to MSB-first extraction.

**How to avoid:** The inner loop is:
```rust
for _ in 0..4 {
    let bits = byte & 0x3;
    byte >>= 2;
    // ...
}
```
Not:
```rust
for i in (0..4).rev() {
    let bits = (byte >> (i * 2)) & 0x3;
    // ...
}
```

**Warning signs:** Identicon visual output differs from OpenSSH `ssh-keygen -lv` on the same key fingerprint.

### Pitfall 4: sha2 GenericArray to [u8; 32] Conversion

**What goes wrong:** `Sha256::digest(...)` returns `GenericArray<u8, U32>`, not `[u8; 32]`. Attempting `let x: [u8; 32] = Sha256::digest(...)` fails without `.into()`.

**How to avoid:** Use `.into()` explicitly:
```rust
let fingerprint: [u8; 32] = Sha256::digest(pubkey_bytes).into();
```

### Pitfall 5: ProjectDirs::from Returns Option

**What goes wrong:** `ProjectDirs::from("", "", "periphore")` returns `Option<ProjectDirs>`. Calling `.unwrap()` panics on systems with no home directory (e.g., containers with `UID=0` and no home).

**How to avoid:** Return `IdentityError::NoDataDir` when `ProjectDirs::from` returns `None`. The `load_or_create(path)` API takes an explicit `&Path` — path resolution is the caller's responsibility. Provide `default_key_path() -> Option<PathBuf>` as a separate helper, used only by `periphored`.

### Pitfall 6: File Written Before Permissions Set

**What goes wrong:** Between `fs::write(path, seed)` and `fs::set_permissions(path, 0o600)` there is a window where the file is world-readable (default umask may set 0644 or 0666).

**Why it matters:** The seed file is a 32-byte private key. Even a brief window of world-readability is a security exposure.

**How to avoid:** Use `OpenOptions` with an initial restricted mode (not available cross-platform without `PermissionsExt`), or accept the brief window and `set_permissions` immediately after write with no intervening `await` points. Since `IdentityStore::load_or_create` is called before any async tasks are spawned, the risk is minimal in practice but should be documented.

**Better alternative:** Create the file with O_EXCL and set permissions before writing data:
```rust
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;

let mut file = OpenOptions::new()
    .write(true)
    .create_new(true)
    .mode(0o600)
    .open(path)?;
file.write_all(&seed)?;
```
This sets mode 0600 atomically at creation via `OpenOptionsExt::mode()`. [VERIFIED: `std::os::unix::fs::OpenOptionsExt` is a stable trait]

### Pitfall 7: send_ok Exhaustiveness Breaks When IpcResponse Changes

**What goes wrong:** Adding `Identicon` and `WordPhrase` variants to `IpcResponse` will cause the existing `roundtrip.rs` tests and `send_ok` in `periphored/main.rs` to compile with a warning (non-exhaustive match patterns if wildcard arms exist) or break tests that enumerate all variants.

**How to avoid:** The roundtrip test in `periphore-protocol/tests/roundtrip.rs` tests `IpcResponse` serialization — it will need new test cases for the new variants. The `send_ok` function uses a wildcard `_ => {}` arm for unhandled variants — this will silently accept the new variants without warning. Moving `GetIdenticon` and `GetWordPhrase` dispatch out of `send_ok` and into the main `select!` loop avoids this.

### Pitfall 8: BIP39 Wordlist Provenance

**What goes wrong:** Using the wrong word list (e.g., a 2047-word list, or a list with different word ordering) breaks cross-platform determinism.

**How to avoid:** Source the list from the canonical BIP39 repository: https://github.com/trezor/python-mnemonic/blob/master/src/mnemonic/wordlist/english.txt — exactly 2048 words, alphabetically sorted. Verify `BIP39_WORDS.len() == 2048` with a compile-time assertion:
```rust
const _: () = assert!(BIP39_WORDS.len() == 2048);
```

---

## Plan Breakdown Recommendation

Phase 2 is well-suited to 3 plans:

### Plan 02-01: Foundation (Wave 1)
**Scope:**
- Add `rand_core` to workspace deps + enable `rand_core` feature on `ed25519-dalek`
- Add `periphore-identity` Cargo.toml deps (`rand_core`, `thiserror`, `directories`)
- Add `IdentityError` enum with `thiserror` derive
- Implement `IdentityStore::load_or_create` (keypair gen, file write with 0o600, load, corrupt error)
- Implement `fingerprint_hex()`
- Add `default_key_path()` free function using `ProjectDirs`
- Add `periphore-identity = { workspace = true }` to `periphored/Cargo.toml`
- Add `IdentityStore` load in `periphored/main.rs` startup (before IPC loop)
- Fill in `GetStatus` fingerprint field
- Compile check: `cargo build --workspace`

**Tests:** `test_first_run_creates_key_file`, `test_load_after_create_is_identical`, `test_corrupt_key_file_error`, `test_fingerprint_determinism`

### Plan 02-02: Visual and Verbal Identity (Wave 2)
**Scope:**
- Inline BIP39 wordlist (`src/bip39.rs`) with compile-time length assertion
- Implement `word_indices()` (6 x 11-bit extraction from 32 bytes)
- Implement `word_phrase()` on `IdentityStore`
- Implement `identicon()` (Drunken Bishop algorithm, header/footer format per D-06)
- Compute golden values and add golden-value tests

**Tests:** `test_word_phrase_determinism`, `test_word_phrase_validity`, `test_identicon_determinism`, `test_identicon_borders`, `test_identicon_line_count`

### Plan 02-03: IPC Integration (Wave 3)
**Scope:**
- Add `Identicon` and `WordPhrase` variants to `IpcResponse` in `periphore-protocol`
- Move `GetIdenticon` and `GetWordPhrase` dispatch from `send_ok()` to main `select!` arms
- Update `periphore-protocol/tests/roundtrip.rs` with new `IpcResponse` variant round-trips
- Add `IdentityConfig` and `identity.show_identicon` to `periphore-config` schema
- Add config test for identity section defaults
- Full workspace compile + test pass

**Tests:** `IpcResponse::Identicon` round-trip, `IpcResponse::WordPhrase` round-trip, `test_identity_config_defaults`

---

## Sources

### Primary (HIGH confidence)
- `~/.cargo/registry/src/.../ed25519-dalek-2.2.0/src/signing.rs` — `SigningKey::from_bytes`, `to_bytes`, `generate`, `verifying_key` signatures
- `~/.cargo/registry/src/.../rand_core-0.6.4/src/os.rs` — `OsRng` struct and `fill_bytes` via `getrandom`
- `~/.cargo/registry/src/.../directories-6.0.0/src/{lib,mac,lin}.rs` — `ProjectDirs::from`, `data_dir()` paths for macOS and Linux
- `~/.cargo/registry/src/.../sha2-0.10.9/src/lib.rs` — one-shot `Sha256::digest` API
- `crates/periphored/src/main.rs` — IPC dispatch structure, `send_ok`, stub arms for `GetIdenticon`/`GetWordPhrase`
- `crates/periphore-protocol/src/ipc.rs` — current `IpcResponse` variants (confirmed missing `Identicon`/`WordPhrase`)
- `crates/periphore-config/src/schema.rs` — current config struct (confirmed no `IdentityConfig`)
- crates.io API — ed25519-dalek 2.2.0 features (rand_core optional), rand 0.8.6 (uses rand_core 0.6), rand 0.9.x incompatibility

### Secondary (MEDIUM confidence)
- Python simulation of Drunken Bishop algorithm — verified header/footer format matches D-06, verified 19-char line width, verified algorithm produces correct output
- Python verification of BIP39 11-bit extraction algorithm — cross-validated rust-style sliding-window vs big-integer approach across 5 random inputs

### Tertiary (LOW confidence)
- OpenSSH Drunken Bishop bit order (LSB-first within byte) — [ASSUMED from OpenSSH convention; should be cross-validated against `ssh-keygen -lv` output on a known key during implementation]

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Start/end position overlap: `E` takes priority over `S` (follows OpenSSH) | Implementation Strategy §4 | Visual inconsistency vs OpenSSH output; low impact |
| A2 | BIP39 wordlist source is trezor/python-mnemonic english.txt | Implementation Strategy §5 | Wrong words if different ordering used — cross-platform determinism breaks |
| A3 | Drunken Bishop bit order is LSB-first within each byte | Pitfall 3 | Identicon differs from OpenSSH; identity verification fails visually |
| A4 | `mode(0o600)` via `OpenOptionsExt` on macOS/Linux creates file with exact 0600 mode (umask not applied) | Pitfall 6 | Slightly wrong permissions on some systems; security exposure |

**Risk mitigation:** A3 (bit order) and A2 (wordlist) are the highest-risk assumptions. Both should be verified during Wave 1/2 implementation with a cross-check against known test vectors.

---

## Open Questions

1. **Should `identicon()` accept an optional width override for future flexibility?**
   - What we know: D-05 locks the grid to 17×9. Context says "Claude decides."
   - Recommendation: No override parameter for now. The method signature `fn identicon(&self) -> String` is cleaner, and future flexibility can be added as a separate method or builder pattern without breaking the API. Keep it simple for Phase 2.

2. **Should the BIP39 word list be in a separate file or inlined with a `include_str!` macro?**
   - What we know: D-11 says "inlined as static." The wordlist is ~16KB.
   - Recommendation: Use a separate `src/bip39.rs` with the `static` declaration. Do NOT use `include_str!` + parsing at startup — precompute the `&[&str; 2048]` directly in source. This avoids any runtime parsing overhead and makes it a compile-time constant.

3. **What happens if `periphored` is run as root (no home directory)?**
   - What we know: `ProjectDirs::from` returns `None` when no home dir is found.
   - Recommendation: Return `IdentityError::NoDataDir` and exit with a clear error message. Document that periphored should not run as root. Existing PITFALLS.md P3 already prohibits root execution for evdev.

---

## Environment Availability

Step 2.6: SKIPPED — this phase is code-only changes. No external tools, services, or CLIs are required beyond the Rust toolchain already verified to be functional (Phase 1 baseline passes).

---

## Security Domain

`security_enforcement` is enabled in `.planning/config.json` with `security_asvs_level: 1`.

### Applicable ASVS Categories (Level 1)

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No — no user auth in this phase | — |
| V3 Session Management | No | — |
| V4 Access Control | Partial — key file access control | `PermissionsExt::from_mode(0o600)` or `OpenOptionsExt::mode(0o600)` |
| V5 Input Validation | Partial — key file length validation | `IdentityError::CorruptKeyFile(len)` on wrong-length read |
| V6 Cryptography | Yes — key generation uses CSPRNG | `OsRng` via `getrandom` (OS entropy); Ed25519 via audited `ed25519-dalek` |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Private key file world-readable | Information Disclosure | `OpenOptionsExt::mode(0o600)` at creation, not post-write |
| Weak entropy during keypair generation | Spoofing | `OsRng` (getrandom — OS entropy), not `StdRng` or user-supplied seed |
| Key file tampering / truncation | Tampering | Validate 32-byte length; return `CorruptKeyFile` error on mismatch |
| Key path traversal | Tampering | Use `ProjectDirs` — OS-defined path, not user-controlled input |

---

## Metadata

**Confidence breakdown:**
- ed25519-dalek API: HIGH — verified from local source in cargo registry
- rand_core/OsRng integration: HIGH — verified from local source + crates.io dependency graph
- directories 6.0 API + paths: HIGH — verified from local source with macOS and Linux platform files
- sha2 one-shot API: HIGH — verified from local source
- Drunken Bishop algorithm: MEDIUM-HIGH — Python simulation verified; bit order is ASSUMED LSB-first (A3)
- BIP39 extraction algorithm: HIGH — cross-validated via Python
- IPC/config integration points: HIGH — verified from current codebase

**Research date:** 2026-04-22
**Valid until:** 2026-05-22 (stable crates, 30-day window)
