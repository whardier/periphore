# Phase 3, Plan 01 — Summary: periphore-trust Scaffold

## What was done

Wave 0 scaffold for the `periphore-trust` crate. All types and test stubs compile; method bodies are `todo!()` placeholders to be filled in Plan 03.

## Files created / modified

### Modified
- `Cargo.toml` (workspace root)
  - Added `periphore-trust = { path = "crates/periphore-trust", version = "0.1.0" }` to `[workspace.dependencies]`
  - Added `toml = { version = "0.8", features = ["display"] }` and `tempfile = { version = "3" }` as workspace-level external deps (promoted from periphore-identity's bare dev-dep)
- `crates/periphore-config/tests/config.rs` — appended 4 stub tests for CFG-02 (PeerConfig.name) and CFG-03 (TopologyConfig.monitors)

### Created
- `crates/periphore-trust/Cargo.toml` — crate manifest with workspace inheritance
- `crates/periphore-trust/src/lib.rs` — public API surface: re-exports from `store`, `default_trust_path()`
- `crates/periphore-trust/src/store.rs` — `TrustError`, `TrustedPeer`, `TrustStore`, `check_peer_fingerprint` (all stub bodies)
- `crates/periphore-trust/tests/trust.rs` — 10 integration test stubs (7 SEC-05, 3 SEC-06)

## Verification results

All three checks exited 0:
1. `cargo test -p periphore-trust --test trust --no-run` — compiled cleanly (warnings only, expected for stubs)
2. `cargo test -p periphore-config --test config --no-run` — compiled cleanly
3. `cargo build --workspace` — full workspace builds successfully

## Notes

- Warnings in `periphore-trust` are expected: unused variables and dead field are all due to `todo!()` stub bodies; they do not affect compilation or correctness.
- Pre-existing warnings in `periphore-config` tests (unsafe env-var blocks) are pre-existing and unrelated to this plan.
