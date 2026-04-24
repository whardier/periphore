# Phase 03, Plan 04 — Wire periphore-trust into periphored

## What was done

1. **Cargo.toml** (`crates/periphored/Cargo.toml`): Added `periphore-trust = { workspace = true }` to `[dependencies]` (alphabetical order among internal crates). Updated `[dev-dependencies]` to use `tempfile = { workspace = true }` (promoted in Plan 01).

2. **Trust cache startup** (`crates/periphored/src/main.rs`): Inserted a trust cache initialization block immediately after the identity load block. `periphore_trust::default_trust_path()` resolves the XDG data dir path; `TrustStore::load` returns an empty cache on first run (file not found) or deserializes the existing TOML. Variable is `mut` because `add_trusted` requires `&mut self`. `trust_path` stays in scope for the full event loop.

3. **IPC dispatch — AcceptFingerprint**: Added a named `select!` arm that calls `trust_store.add_trusted(&fingerprint, None, &trust_path)`. On success responds `IpcResponse::Ok`; on error responds `IpcResponse::Error { message }`.

4. **IPC dispatch — RejectFingerprint**: Added a named `select!` arm that logs the rejection and responds `IpcResponse::Ok` with no state change (rejection is stateless by design).

5. **send_ok cleanup**: Removed the two stub arms for `AcceptFingerprint` and `RejectFingerprint` from `send_ok`. Updated the comment above the wildcard arm to list both commands as having dedicated select arms.

## Verification

- `cargo build -p periphored` — exits 0, no warnings on periphored itself.
- `cargo test --workspace` — 46 tests across all crates, all pass.
- Grep checks confirm `AcceptFingerprint`/`RejectFingerprint` appear in the select loop but not inside `send_ok`; `trust_store` and `periphore_trust` both present in main.rs.
