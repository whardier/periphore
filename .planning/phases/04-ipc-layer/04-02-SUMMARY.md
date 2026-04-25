---
phase: 04-ipc-layer
plan: "02"
subsystem: daemon
tags: [rust, tracing-subscriber, reload, sighup, ipc, config]

# Dependency graph
requires:
  - phase: 04-ipc-layer/04-01
    provides: CR-01 fix — clean JoinSet exit handling in periphored main loop
  - phase: 03-configuration-trust-persistence
    provides: periphore_config::load(), Config struct with daemon/logging fields

provides:
  - Runtime config reload via SIGHUP signal in periphored
  - Runtime config reload via ReloadConfig IPC command in periphored
  - Hot-reload of logging level without daemon restart (tracing_subscriber::reload::Layer)
  - Warnings for restart-required fields (daemon.socket_path, daemon.port) on reload

affects: [04-03-periphore-core, 05-cli-layer, 06-tcp-peering]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "tracing_subscriber::reload::Layer wrapping EnvFilter for runtime log-level updates"
    - "Free generic function reload_config<S: tracing::Subscriber> for handle-passing without type aliases"
    - "Reload-failure isolation: None return keeps existing config; daemon never crashes on bad file"

key-files:
  created: []
  modified:
    - crates/periphored/src/main.rs

key-decisions:
  - "Use reload::Layer (not FmtSubscriber::builder) so filter_handle can update log level at runtime (D-03)"
  - "reload_config free function with <S: tracing::Subscriber> generic avoids inline duplication while compiling cleanly"
  - "daemon.socket_path and daemon.port warn but are not applied on reload — require daemon restart (D-02)"
  - "On parse failure, return None and keep existing config — IPC responds Error, daemon continues (D-04)"
  - "Identity and trust store not reloaded on SIGHUP/ReloadConfig (D-05)"

patterns-established:
  - "Pattern: runtime filter reload via tracing_subscriber::reload::Handle stored in main() scope"
  - "Pattern: reload isolation — errors log and return None; caller retains old value"

requirements-completed: [IPC-01, IPC-02]

# Metrics
duration: 3min
completed: 2026-04-25
---

# Phase 04 Plan 02: Config Reload Summary

**SIGHUP and ReloadConfig IPC both reload config from disk using tracing_subscriber::reload::Layer for hot log-level updates and restart-required field warnings**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-04-25T09:46:27Z
- **Completed:** 2026-04-25T09:48:45Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- Restructured periphored tracing subscriber from `FmtSubscriber::builder()` to `registry() + reload::Layer` pattern, exposing a `filter_handle` for runtime log-level changes
- Implemented `reload_config<S>` free function: loads config via `periphore_config::load()`, hot-reloads tracing filter on `logging.level` change, warns on restart-required field changes, returns `None` on failure
- Wired SIGHUP arm: replaced Phase 4 placeholder with real `reload_config` call that replaces the in-memory config binding
- Wired ReloadConfig IPC arm: returns `IpcResponse::Ok` on success, `IpcResponse::Error` on parse failure; daemon never crashes on reload failure

## Task Commits

Each task was committed atomically:

1. **Task 1: Restructure tracing subscriber to reload::Layer** - `95a4cfb` (refactor)
2. **Task 2: Implement reload_config and wire SIGHUP + ReloadConfig arms** - `ac70863` (feat)

## Files Created/Modified

- `crates/periphored/src/main.rs` — tracing subscriber restructured; `reload_config<S>` free function added; SIGHUP and ReloadConfig IPC arms wired with real reload logic; `config` binding made `mut`

## Decisions Made

- Used `<S: tracing::Subscriber>` generic on `reload_config` instead of inline duplication — the bound compiles cleanly because the registry's concrete type satisfies `Subscriber`; avoids repeating reload logic in two select! arms
- `SubscriberExt` and `SubscriberInitExt` imported as `_` inside `main()` to avoid polluting the module namespace with unused trait names
- `daemon.socket_path` and `daemon.port` checked for changes and warned but not applied — changing socket path at runtime would break existing connected IPC clients

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `filter_handle` and `reload_config` are fully operational; Phase 5 CLI can add a `--reload` subcommand that sends ReloadConfig IPC
- `daemon.port` reload warning is wired; Phase 6 TCP peering will consume `config.daemon.port` on startup, consistent with restart-required semantics
- No blockers for Phase 04-03 (periphore-core state machine)

---

*Phase: 04-ipc-layer*
*Completed: 2026-04-25*

## Self-Check: PASSED

- File exists: `crates/periphored/src/main.rs` — FOUND
- Commit `95a4cfb` exists — FOUND
- Commit `ac70863` exists — FOUND
