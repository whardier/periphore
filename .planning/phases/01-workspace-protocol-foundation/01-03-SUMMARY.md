---
phase: 01-workspace-protocol-foundation
plan: 03
subsystem: config
tags: [rust, figment, serde, toml, config, layering, cfg-01]

# Dependency graph
requires:
  - phase: 01-workspace-protocol-foundation
    provides: Cargo workspace with periphore-config crate stub and dependencies
provides:
  - "Full config schema: Config, DaemonConfig, LoggingConfig, PeerConfig, TopologyConfig, CaptureConfig"
  - "Figment layered load() function with correct merge order (defaults < TOML < env)"
  - "CFG-01 compile-time enforcement: Config has no Serialize derive, no write paths in crate"
  - "5 integration tests verifying all layering levels and edge cases"
affects: [01-04, 01-05, 01-06, phase-03, phase-05, phase-06]

# Tech tracking
tech-stack:
  added: [tempfile 3 (dev-dependency)]
  patterns: [figment-defaults-via-serde-default, env-mutex-for-test-isolation, no-serialize-config-invariant]

key-files:
  created:
    - crates/periphore-config/src/schema.rs
    - crates/periphore-config/tests/config.rs
  modified:
    - crates/periphore-config/src/lib.rs
    - crates/periphore-config/Cargo.toml

key-decisions:
  - "Defaults provided via #[serde(default)] + Default impls instead of Figment Serialized::defaults, avoiding Serialize requirement on Config (preserves CFG-01)"
  - "ENV_MUTEX used in tests to serialize access to PERIPHORE_* env vars, preventing race conditions in parallel test execution"
  - "tempfile added as direct dev-dependency (not workspace) since it is test-only for this crate"

patterns-established:
  - "Config no-Serialize invariant: Config derives only Deserialize + Default, never Serialize (CFG-01)"
  - "Figment defaults via serde: use Figment::new() with #[serde(default)] on all fields instead of Serialized::defaults"
  - "Env var test isolation: static Mutex + clear_periphore_env() helper before each test that reads config"

requirements-completed: [CFG-01]

# Metrics
duration: 4min
completed: 2026-04-23
---

# Phase 1 Plan 03: Config Schema Summary

**Figment layered config loading with full schema, env var override, and compile-time no-Serialize enforcement (CFG-01)**

## Performance

- **Duration:** 4 min
- **Started:** 2026-04-23T02:11:45Z
- **Completed:** 2026-04-23T02:16:21Z
- **Tasks:** 1 (TDD: RED + GREEN)
- **Files created:** 2
- **Files modified:** 2

## Accomplishments
- Full config schema with all 5 top-level sections: DaemonConfig (socket_path, port), LoggingConfig (level with "info" default), PeerConfig (fingerprint, host, port), TopologyConfig (placeholder), CaptureConfig (placeholder)
- Figment load() with correct merge order: Figment::new() -> merge(Toml) -> merge(Env) ensuring env vars override TOML which overrides defaults
- CFG-01 enforced at compile time: Config has no Serialize derive, no fs::write/File::create/BufWriter in crate src/
- 5 integration tests covering defaults, TOML override, env override, missing file graceful handling, and empty peers default

## Task Commits

Each task was committed atomically (TDD flow):

1. **Task 1 RED: Add failing config layering tests** - `ac693ec` (test)
2. **Task 1 GREEN: Implement config schema and Figment layered loading** - `f2b0556` (feat)

## Files Created/Modified
- `crates/periphore-config/src/schema.rs` - Full config schema: Config, DaemonConfig, LoggingConfig, PeerConfig, TopologyConfig, CaptureConfig with Deserialize + Default derives (no Serialize)
- `crates/periphore-config/src/lib.rs` - load() function with Figment merge chain, ConfigError type, public re-exports of all config types
- `crates/periphore-config/tests/config.rs` - 5 integration tests: defaults_load_without_file, toml_file_overrides_defaults, env_overrides_toml_file, missing_toml_file_is_ignored, peer_config_vec_defaults_to_empty
- `crates/periphore-config/Cargo.toml` - Added tempfile as dev-dependency for TOML test fixtures

## Decisions Made
- Used `Figment::new()` with `#[serde(default)]` on all Config fields instead of `Serialized::defaults(Config::default())` because the latter requires Config to implement Serialize, which would violate CFG-01. Serde fills in missing keys from Default impls when extracting from an empty or partial Figment provider. This achieves the same layering behavior without compromising the no-Serialize invariant.
- Added a static `ENV_MUTEX` in tests to serialize access to PERIPHORE_* environment variables. Rust edition 2024 makes `set_var`/`remove_var` unsafe (they are inherently thread-unsafe), and Figment reads env vars on every `load()` call. The mutex prevents concurrent tests from interfering with each other's env state.
- The `tempfile` crate was added directly to `[dev-dependencies]` (not as a workspace dependency) since it is only used for test fixtures in this crate.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Replaced Serialized::defaults with Figment::new() + #[serde(default)]**
- **Found during:** Task 1 GREEN (implementation)
- **Issue:** The plan specified `Figment::from(Serialized::defaults(Config::default()))` but Figment's `Serialized` provider requires the type to implement `serde::Serialize`. Config intentionally has no Serialize derive (CFG-01), so this pattern causes a compilation error.
- **Fix:** Used `Figment::new()` (empty provider) combined with `#[serde(default)]` on all Config fields. Serde's deserialization fills missing keys from Default impls, achieving identical layering behavior without requiring Serialize.
- **Files modified:** `crates/periphore-config/src/lib.rs`
- **Verification:** `cargo test -p periphore-config` passes all 5 tests; `cargo build -p periphore-config` exits 0
- **Committed in:** `f2b0556` (GREEN commit)

**2. [Rule 1 - Bug] Added ENV_MUTEX to prevent env var race conditions in tests**
- **Found during:** Task 1 GREEN (test execution)
- **Issue:** `env_overrides_toml_file` test sets `PERIPHORE_LOGGING_LEVEL=trace` which leaked to `toml_file_overrides_defaults` running concurrently in another thread, causing assertion failure (`"trace" != "debug"`)
- **Fix:** Added a `static ENV_MUTEX: Mutex<()>` that all tests acquire before calling `load()`, plus a `clear_periphore_env()` helper called at the start of each test
- **Files modified:** `crates/periphore-config/tests/config.rs`
- **Verification:** `cargo test -p periphore-config` passes all 5 tests consistently across multiple runs
- **Committed in:** `f2b0556` (GREEN commit)

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both fixes were necessary for correctness. The Serialized::defaults pattern from RESEARCH.md is fundamentally incompatible with the no-Serialize CFG-01 invariant -- the replacement achieves identical behavior. The env mutex fix prevents non-deterministic test failures. No scope creep.

## Issues Encountered
None beyond the deviations documented above.

## User Setup Required
None - no external service configuration required.

## TDD Gate Compliance
- RED: `ac693ec` -- tests/config.rs written first, failed to compile (`no 'load' in the root`) -- confirmed failing
- GREEN: `f2b0556` -- schema.rs + lib.rs implemented, all 5 tests pass -- confirmed passing
- REFACTOR: not needed -- code is clean as written

## Next Phase Readiness
- Config crate fully functional; all subsequent crates can call `periphore_config::load()`
- Ready for Plan 04 (IPC implementation) and Plan 05 (daemon wiring) which consume this config
- No blockers or concerns

## Self-Check: PASSED

- `crates/periphore-config/src/schema.rs` -- FOUND
- `crates/periphore-config/src/lib.rs` -- FOUND
- `crates/periphore-config/tests/config.rs` -- FOUND
- Commit `ac693ec` (RED) -- FOUND
- Commit `f2b0556` (GREEN) -- FOUND
- `cargo test -p periphore-config` -- 5 passed, 0 failed
- `cargo build -p periphore-config` -- exits 0
- No Serialize in derives -- confirmed
- No write paths in src/ -- confirmed

---
*Phase: 01-workspace-protocol-foundation*
*Completed: 2026-04-23*
