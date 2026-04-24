# Plan 03-03 Summary: TrustStore Implementation

## What was done

Implemented all `todo!()` stubs in `crates/periphore-trust/src/store.rs` and replaced test stubs in `crates/periphore-trust/tests/trust.rs`.

## Implementation details

**`store.rs`** — replaced 5 stubs + added private `save` method:
- `TrustStore::load` — reads TOML from disk if present, returns empty cache if missing
- `TrustStore::is_trusted` — case-insensitive fingerprint lookup via `to_ascii_lowercase()`
- `TrustStore::add_trusted` — idempotent (updates alias if already present, no duplicate entries)
- `TrustStore::remove_trusted` — returns `TrustError::NotFound` if fingerprint not in cache
- `TrustStore::save` (private) — atomic write via `tempfile::NamedTempFile::new_in` + `persist()` rename; creates parent dirs on first run; calls `sync_all()` before rename for durability
- `check_peer_fingerprint` — pure function, case-insensitive comparison, returns `FingerprintConflict` on mismatch

**`tests/trust.rs`** — 10 integration tests implemented:
- SEC-05: persistence (add/reload, missing file, corrupt file, cache path separation, idempotent add, remove, remove-nonexistent)
- SEC-06: conflict detection (match, mismatch, case-insensitive)

## Verification

- `cargo test -p periphore-trust --test trust` — 10/10 passed
- `grep -c "todo!"` — 0 in both files
- `cargo test --workspace` — periphore-trust clean; periphore-config has 5 pre-existing failures from Plan 03-01/03-02 schema additions not yet fully implemented (unrelated to this plan)

## Key decisions

- Idempotent `add_trusted`: returns `Ok(())` if fingerprint already present (updates alias if provided), matching RESEARCH.md recommendation for better operator UX
- Atomic write: same-filesystem tempfile + rename prevents partial writes visible to other processes
- Case normalization: stored as lowercase, compared as lowercase throughout
