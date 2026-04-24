# Phase 4: IPC Layer — Context

**Gathered:** 2026-04-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 4's stated requirements (IPC-01, IPC-02) were delivered in Phase 1. Phase 4 is retained
for IPC enhancements per Phase 1 CONTEXT D-20. This phase delivers three concrete items:

1. **Full config reload** — SIGHUP + `ReloadConfig` IPC re-reads all config fields from disk without
   daemon restart. Fields that cannot take effect until restart are logged as warnings.
2. **periphore-core state machine** — Implement the focus/transfer state machine as a standalone
   library (zero platform deps, pure logic, fully unit-testable). Not wired into periphored yet —
   Phase 6 (TCP peering) adopts it when real peers exist.
3. **CR-01 fix** — Eliminate the 100% CPU spin when the IPC task exits cleanly (JoinSet::join_next
   guard + correct break logic).

**Out of scope for Phase 4:**
- Wiring periphore-core into periphored (Phase 6)
- Additional IPC commands beyond ReloadConfig (Phase-specific plans add those)
- CLI subcommands (Phase 5)
- TCP peering (Phase 6)

</domain>

<decisions>
## Implementation Decisions

### Config Reload (ReloadConfig IPC + SIGHUP)
- **D-01:** Full config reload — re-read all fields from disk on `ReloadConfig` IPC or `SIGHUP`.
  Replace the daemon's in-memory `config` binding with the newly-loaded struct.
- **D-02:** Fields that require restart to take effect (e.g., `socket_path` change, cert rotation)
  are logged at `warn` level: `"config field X changed but requires restart to take effect"`. No
  error, no abort — daemon continues with old value for those fields.
- **D-03:** Logging level reload uses `tracing_subscriber`'s `reload::Layer` so the subscriber
  filter can be updated at runtime without reinitializing the global subscriber. This is the only
  field that needs special reload machinery.
- **D-04:** `ReloadConfig` IPC responds `IpcResponse::Ok` on success, `IpcResponse::Error` on
  parse failure. The daemon does NOT crash on reload failure — it logs the error and keeps
  the existing config.
- **D-05:** No reload of identity or trust store on SIGHUP/ReloadConfig — those are loaded once
  at startup and managed by their own IPC commands (AcceptFingerprint etc.).

### periphore-core State Machine
- **D-06:** `periphore-core` implements the focus/transfer state machine as a pure Rust library
  (no `async`, no platform deps, no I/O). Phases 6, 8, 9 adopt it when they need routing logic.
- **D-07:** Focus state enum (two states):
  ```rust
  pub enum FocusState {
      LocalFocus,
      ForwardingTo { peer_id: PeerId },
  }
  ```
  `PeerId` is a newtype wrapping the fingerprint hex string (`String`) — unique peer identity.
- **D-08:** `FocusStateMachine` struct owns the current state and exposes pure transition methods:
  - `transfer_to(peer_id) -> Result<(), FocusError>` — transitions LocalFocus → ForwardingTo
  - `reclaim() -> Result<(), FocusError>` — transitions ForwardingTo → LocalFocus
  - `current_state() -> &FocusState`
  - Returns `FocusError::AlreadyForwarding` / `FocusError::NotForwarding` for invalid transitions
- **D-09:** `periphore-core` does NOT depend on `periphore-protocol` in Phase 4. Types are
  defined locally; Phase 6 aligns them with protocol types when the connection exists.
- **D-10:** `periphore-core` is NOT added as a dependency of `periphored` in Phase 4. It is a
  standalone library with its own tests. Phase 6 adds the dep.

### CR-01 Fix (JoinSet CPU spin)
- **D-11:** Fix the `JoinSet::join_next()` spin: add `if tasks.is_empty()` guard before the
  `join_next` branch. When the JoinSet is empty, `join_next()` returns `Poll::Pending` forever,
  causing the select! loop to spin. Guard prevents the branch from being polled when there are
  no tasks.
- **D-12:** Also add `break` on `Some(Ok(Ok(())))` (clean IPC task exit) so the daemon shuts
  down cleanly when the IPC server exits without error, rather than entering an empty-JoinSet spin.

### Claude's Discretion
- Exact tracing-subscriber `reload::Layer` wiring (EnvFilter vs LevelFilter — Claude picks
  whichever integrates cleanly with the existing `FmtSubscriber` setup)
- Whether config reload triggers a single tracing::info event summarizing what changed
- `PeerId` newtype internal representation — String (fingerprint hex) is the spec; wrapper style
  is Claude's choice

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Prior Phase Context
- `.planning/phases/01-workspace-protocol-foundation/01-CONTEXT.md` — D-18 (socket lifecycle),
  D-19 (InjectInputEvent/SimulateEdgeCross as test backbone), D-20 (Phase 4 retained for enhancements)
- `.planning/phases/03-configuration-trust-persistence/03-CONTEXT.md` (if exists) — trust store
  decisions, config field inventory

### Codebase
- `crates/periphored/src/main.rs` — current SIGHUP placeholder (line ~134), ReloadConfig arm
  (line ~195), JoinSet logic (lines ~214–231), send_ok function
- `crates/periphore-core/src/lib.rs` — current 2-line stub to be replaced
- `crates/periphore-config/src/schema.rs` — config struct fields to understand reload scope
- `crates/periphore-identity/Cargo.toml` — pattern to replicate for periphore-core Cargo.toml

### Requirements
- `.planning/REQUIREMENTS.md` §IPC (IPC-01, IPC-02 — already complete, no new IPC requirements)
- `.planning/STATE.md` — CR-01 open TODO (JoinSet spin bug), exact description

### Architecture
- `.planning/research/ARCHITECTURE.md` — channel topology, IPC design rationale

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `periphore-identity` crate structure — exact pattern to replicate for `periphore-core` (Cargo.toml,
  `[lib] test = false`, integration tests in `tests/`)
- Existing IpcCommand dispatch arms in `periphored/src/main.rs` — ReloadConfig arm is already
  wired to select!, just needs real implementation

### Established Patterns
- `thiserror`-derived error enums in library crates — `FocusError` follows this
- Integration tests in `tests/` subdir with `[lib] test = false` — periphore-core must match
- `tracing::info!` / `tracing::warn!` for runtime events in daemon
- `anyhow` at daemon boundary, `thiserror` in library crates

### Integration Points (Phase 4)
- `periphored/src/main.rs` ReloadConfig arm: calls `periphore_config::load()`, replaces config
- `periphored/src/main.rs` SIGHUP arm: same reload path as ReloadConfig
- `periphored/src/main.rs` JoinSet block: add `if !tasks.is_empty()` guard + clean-exit break
- `crates/periphore-core/src/lib.rs`: replace stub with real state machine

### Integration Points (Phase 6, future)
- `periphored` will add `periphore-core` dep and route `SimulateEdgeCross` through `FocusStateMachine`
- `periphore-core::PeerId` will align with `periphore-protocol` peer identity types

</code_context>

<specifics>
## Specific Ideas

- Config reload should use the same `periphore_config::load(config_path)` call as startup —
  no special reload path, just re-run the same load function
- The tracing reload layer should use `tracing_subscriber::reload::Layer` wrapping an
  `EnvFilter` — this is the standard pattern for runtime filter updates
- periphore-core has zero external deps beyond std — no tokio, no serde, no platform crates.
  It is the pure business logic layer.

</specifics>

<deferred>
## Deferred Ideas

- Wiring periphore-core into periphored — Phase 6 (when real peers trigger focus transfers)
- `SimulateEdgeCross` routing through FocusStateMachine — Phase 6 or Phase 8
- Additional focus states (Reclaiming, multi-peer arbitration) — Phase 8/9 as complexity emerges
- Hot-reload of peer list without restart — Phase 6 (requires connection management)

</deferred>

---

*Phase: 04-ipc-layer*
*Context gathered: 2026-04-24*
