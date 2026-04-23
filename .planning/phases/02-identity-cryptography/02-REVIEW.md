---
phase: 02-identity-cryptography
reviewed: 2026-04-22T00:00:00Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - Cargo.toml
  - crates/periphore-config/src/lib.rs
  - crates/periphore-config/src/schema.rs
  - crates/periphore-config/tests/config.rs
  - crates/periphore-identity/Cargo.toml
  - crates/periphore-identity/src/bip39.rs
  - crates/periphore-identity/src/lib.rs
  - crates/periphore-identity/tests/identity.rs
  - crates/periphore-ipc/tests/socket.rs
  - crates/periphore-protocol/src/ipc.rs
  - crates/periphore-protocol/tests/roundtrip.rs
  - crates/periphored/Cargo.toml
  - crates/periphored/src/main.rs
findings:
  critical: 1
  warning: 3
  info: 3
  total: 7
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-04-22T00:00:00Z
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

Phase 2 implements Ed25519 keypair lifecycle (`IdentityStore`), SHA-256 fingerprinting, Drunken Bishop identicon, BIP39 word phrase, and IPC wiring for identity queries. The cryptographic foundations are solid: `OsRng` is used correctly, the atomic key-file creation with `mode(0o600)` eliminates the world-readable race window, and the Drunken Bishop algorithm is correctly implemented (LSB-first byte walk, clamped grid, correct `E`/`S` override priority). The BIP39 bit-extraction logic has been verified correct for all six windows and the compile-time length guard on the wordlist is a good defensive measure.

One critical bug was found in the daemon main loop that would cause a 100% CPU spin under a specific shutdown condition. Three warnings were found: exposed private key material on a public struct field, a latent panic in an internal function, and missing `fsync` after key file creation. Three informational items cover an ignored IPC request field, a fragile environment variable mapping pattern, and a minor test coverage gap.

## Critical Issues

### CR-01: CPU busy-loop when IPC task exits cleanly

**File:** `crates/periphored/src/main.rs:170-184`

**Issue:** When the IPC server task completes successfully (e.g., the socket listener exits without error), the `Some(Ok(Ok(())))` arm logs a message and continues the loop — it does not `break`. After that point `tasks` is empty. In Tokio, `JoinSet::join_next().await` on an empty `JoinSet` is immediately ready and returns `None`. In the `select!` loop the `join_next()` branch wins every poll, spinning the `None => {}` arm at 100% CPU indefinitely. The daemon remains alive but unresponsive to shutdown, and burns a full core until killed externally.

**Fix:**
```rust
// In the task completion arm, break on success too:
Some(Ok(Ok(()))) => {
    tracing::info!("IPC server task completed");
    break; // <-- add this; no tasks remain, nothing left to do
}
// And guard join_next with tasks.is_empty() to avoid the spin entirely:
result = tasks.join_next(), if !tasks.is_empty() => {
    // ... existing match arms
}
```

The cleanest fix is the `if !tasks.is_empty()` precondition on the `join_next` branch (a `select!` precondition disables that branch when false), combined with adding `break` to the success arm so a clean IPC exit causes a graceful shutdown rather than a zombie loop.

## Warnings

### WR-01: Public `keypair` field exposes raw private key material

**File:** `crates/periphore-identity/src/lib.rs:40`

**Issue:** `IdentityStore::keypair` is declared `pub`. Any code with access to an `IdentityStore` can call `identity.keypair.to_bytes()` and extract the 32-byte raw seed — the secret material that must never leave the key file. This is a minimal-authority violation. `SigningKey`'s `Debug` impl is safe (ed25519-dalek 2.x prints `REDACTED`), but direct field access bypasses that protection entirely. As the codebase grows and `IdentityStore` is passed between more modules, this becomes an accidental-disclosure risk.

**Fix:**
```rust
// Make keypair private; expose only the operations the rest of the codebase needs:
pub struct IdentityStore {
    keypair: SigningKey,          // private — never exposed as raw bytes
    pub fingerprint: [u8; 32],
}

impl IdentityStore {
    /// Sign `msg` with this node's private key.
    pub fn sign(&self, msg: &[u8]) -> ed25519_dalek::Signature {
        use ed25519_dalek::Signer;
        self.keypair.sign(msg)
    }

    /// Return a copy of the public (verifying) key.
    pub fn verifying_key(&self) -> ed25519_dalek::VerifyingKey {
        self.keypair.verifying_key()
    }
}
```

Callers in `periphored/src/main.rs` currently only call `fingerprint_hex()`, `identicon()`, and `word_phrase()` — none require direct `keypair` access. Phase 6 handshake code will need `sign()` and `verifying_key()`, which can be added as targeted methods.

### WR-02: `build_border()` panics on labels longer than 13 characters

**File:** `crates/periphore-identity/src/lib.rs:212-215`

**Issue:** `build_border` computes `let dash_count = 13 - label.len()`. Both call sites use hardcoded labels of length 11 and 9, so this cannot panic today. However, `build_border` is a non-`pub` but unrestricted internal function. If a future caller passes a label longer than 13 characters (e.g., a peer name in a future "peer identicon" feature), Rust will panic with a usize underflow in debug builds and wrap/produce garbage in release builds (since `usize` arithmetic is not checked in release). The function provides no indication of this constraint.

**Fix:**
```rust
fn build_border(label: &str) -> String {
    // Panics are acceptable here since both call sites use compile-time constants,
    // but assert documents the invariant clearly:
    assert!(label.len() <= 13, "build_border: label too long ({} > 13 chars)", label.len());
    let dash_count = 13 - label.len();
    format!("+--[{}]{:->width$}+", label, "", width = dash_count)
}
```

Alternatively, use `saturating_sub` and return a fixed-width truncated border, but an assert is simpler and appropriate since this is an internal function with fixed callers.

### WR-03: Key file written without `sync_all` — new identity lost on power failure

**File:** `crates/periphore-identity/src/lib.rs:84-89`

**Issue:** After writing the 32-byte seed with `file.write_all(&seed)`, neither `file.flush()` (which is a no-op for `File` since it writes directly to the OS) nor `file.sync_all()` is called before the file handle is dropped. If the system loses power or the process is killed immediately after `write_all` returns, the OS write buffer may not have reached disk. The result is a zero-byte or partially-written key file that `load_or_create` will detect as `CorruptKeyFile` — the identity is permanently lost. For a long-lived daemon identity this is a meaningful durability gap.

**Fix:**
```rust
use std::os::unix::fs::OpenOptionsExt;
let mut file = std::fs::OpenOptions::new()
    .write(true)
    .create_new(true)
    .mode(0o600)
    .open(path)?;
file.write_all(&seed)?;
file.sync_all()?;   // <-- ensure bytes reach disk before returning
// drop(file) here
```

`sync_all()` also flushes the file metadata (size), which is important because `load_or_create` validates the byte count on subsequent loads.

## Info

### IN-01: `GetIdenticon` and `GetWordPhrase` silently ignore the `fingerprint` IPC field

**File:** `crates/periphored/src/main.rs:139-150`

**Issue:** `IpcRequest::GetIdenticon { fingerprint: String }` and `IpcRequest::GetWordPhrase { fingerprint: String }` accept a `fingerprint` field from IPC clients, implying they can query any peer's identicon/phrase. In the current implementation both arms use `..` to discard the field and always return the daemon's own identity. A client that passes a peer's fingerprint expecting to get that peer's identicon will silently get its own daemon's data instead, with no error or indication that the field was ignored.

**Fix:** Either validate and use the field if the intent is multi-peer lookup, or remove the `fingerprint` field from both `IpcRequest` variants until the feature is implemented (Phase 4+). If keeping the field, return `IpcResponse::Error { message: "peer fingerprint lookup not yet implemented".into() }` when the passed fingerprint does not match the daemon's own.

### IN-02: Env var underscore split constraint is documented but unenforced

**File:** `crates/periphore-config/src/lib.rs:56-63`

**Issue:** The comment accurately documents that `Env::prefixed("PERIPHORE_").split("_")` will silently misroute env vars for any config field whose name contains an underscore. `socket_path` is called out as an exception. The comment relies on future developers reading it before adding underscore-bearing fields. There is no compile-time or runtime guard preventing a future `[daemon]\nreconnect_interval = 5` field from being added and silently broken. This is low urgency now but will become a maintenance trap.

**Fix:** In a future phase, consider switching to a custom Figment env provider that uses a different separator (e.g., `__` double-underscore for nesting), which would allow single underscores in field names. For now, add a test that attempts to set `PERIPHORE_DAEMON_SOCKET_PATH` and asserts the field is NOT populated (confirming the known-broken behavior is at least documented in test form).

### IN-03: Missing key file permission test in identity test suite

**File:** `crates/periphore-identity/tests/identity.rs:20-33`

**Issue:** `test_first_run_creates_key_file` verifies that the key file is created and is 32 bytes, but does not verify the file permissions are `0600`. The socket test suite (`socket_permissions_0600`) has a parallel check for the IPC socket. A missing permission assertion means a regression in the `OpenOptionsExt::mode` call (or its removal in a refactor) would go undetected by the test suite.

**Fix:**
```rust
#[test]
#[cfg(unix)]
fn test_first_run_key_file_permissions_0600() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("temp dir");
    let key_path = dir.path().join("key");
    let _store = IdentityStore::load_or_create(&key_path)
        .expect("load_or_create must succeed on first run");
    let metadata = std::fs::metadata(&key_path).expect("key file metadata");
    let mode = metadata.permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "key file must be 0600, got: {mode:o}");
}
```

---

_Reviewed: 2026-04-22T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
