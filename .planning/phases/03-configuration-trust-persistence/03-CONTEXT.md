# Phase 3: Configuration & Trust Persistence — Context

**Gathered:** 2026-04-23
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 3 delivers the trust infrastructure substrate and config schema groundwork:

1. New `periphore-trust` crate — fingerprint cache (read/write), `TrustStore` API
2. `AcceptFingerprint`/`RejectFingerprint` IPC dispatch wired to real trust cache writes
3. Hard-config fingerprint conflict detection function (SEC-06) — called by Phase 6 during handshake
4. `PeerConfig.name` field added for human-readable peer labels
5. `TopologyConfig` populated with `MonitorConfig` entries for preferred monitor layouts (CFG-03)

**Out of scope for Phase 3:**
- TCP peering handshake that exercises trust enforcement at runtime (Phase 6)
- Topology conflict detection — "both peers claim the same edge" (Phase 8, when edge config exists)
- `periphore monitors list` CLI command to surface available OS monitor IDs (Phase 5/8)
- VNC/RDP as peers without daemon (post-v1 — see Deferred Ideas)

</domain>

<decisions>
## Implementation Decisions

### Trust Cache (SEC-05)

- **D-01:** Cache file path — XDG data home, consistent with the key file (D-02 in Phase 2):
  - Linux: `~/.local/share/periphore/trusted.toml`
  - macOS: `~/Library/Application Support/periphore/trusted.toml`
  - Use `directories::ProjectDirs::data_dir()` — same crate already in workspace.
- **D-02:** Format — TOML. Consistent with main config; human-readable and inspectable. Serde-parseable with existing stack.
- **D-03:** Cache schema — list of `[[trusted]]` entries, each with:
  - `fingerprint: String` — 64-char lowercase hex (required, the identity key)
  - `alias: Option<String>` — user-assigned human label (optional; surfaced in logs; NOT exchanged over the wire)
- **D-04:** Write path — exclusively via `AcceptFingerprint` IPC command. No other code path writes to the trust cache. This is the only new daemon-writes-to-disk path in the entire project.

### periphore-trust Crate (New — 12th crate)

- **D-05:** New `periphore-trust` crate. Trust is distinct from identity (keypair/fingerprint generation). Clean separation of concerns.
- **D-06:** Build order: `protocol → config + identity → trust → core + ipc + cli → net → capture + inject`. Phase 6 TCP peering imports `periphore-trust` directly.
- **D-07:** Public API surface:
  ```rust
  pub struct TrustStore { ... }
  impl TrustStore {
      pub fn load(path: &Path) -> Result<Self, TrustError>;
      pub fn is_trusted(&self, fp: &str) -> bool;
      pub fn add_trusted(&mut self, fp: &str, alias: Option<&str>) -> Result<(), TrustError>;
      pub fn remove_trusted(&mut self, fp: &str) -> Result<(), TrustError>;
  }
  pub struct TrustedPeer {
      pub fingerprint: String,
      pub alias: Option<String>,
  }
  ```
  Minimal surface for Phase 3. Phase 5 may add `.list_trusted() -> &[TrustedPeer]` for CLI display.
- **D-08:** Error type: `thiserror`-derived `TrustError` (consistent with all library crates — same pattern as `IdentityError`).
- **D-09:** File creation on first `add_trusted` — if the cache file doesn't exist, create it atomically. If the file exists but is malformed TOML, return `TrustError::CorruptCacheFile`.

### Peer Naming in Config (SEC-06, CFG-02)

- **D-10:** Add `pub name: Option<String>` to `PeerConfig` in `periphore-config/src/schema.rs`.
- **D-11:** `name` is a **local-only convenience label** — it is NOT sent over the wire, does NOT participate in identity verification, and does NOT need to match between the two machines. The fingerprint is the authoritative peer identity. Documentation MUST make this clear.
- **D-12:** Used in: log messages, error reports, conflict messages. Example: `"fingerprint conflict for peer 'work-mac': expected a3f9..., got b2e1..."`. If `name` is absent, the log uses the fingerprint hex or host address.

### Hard-Config Fingerprint Conflict Detection (SEC-06)

- **D-13:** Phase 3 provides a pure conflict-checking function (in `periphore-trust` or a `periphore-core` module):
  ```rust
  pub fn check_peer_fingerprint(
      configured_fp: &str,
      actual_fp: &str,
      peer_label: &str,  // name or fingerprint prefix for log context
  ) -> Result<(), TrustError>
  ```
  Returns `Err(TrustError::FingerprintConflict { expected, actual, peer_label })` when they don't match.
- **D-14:** Phase 6 (TCP peering) calls `check_peer_fingerprint` during the handshake. On error: `tracing::error!` + drop connection. Phase 3 only delivers the function; Phase 6 wires it into the handshake.
- **D-15:** Topology conflict detection (SC4: both peers claim the same edge) is **deferred to Phase 8**. Phase 3 does not define edge config — no conflict to detect yet.

### TopologyConfig Schema (CFG-03)

- **D-16:** `TopologyConfig` gains a `monitors` field: `Vec<MonitorConfig>`. TOML: `[[topology.monitor]]` entries.
- **D-17:** `MonitorConfig` fields:
  ```rust
  pub struct MonitorConfig {
      pub id: Option<String>,    // free-form OS identifier (xrandr name, EDID fragment, bus path, etc.)
      pub name: Option<String>,  // human-readable label (optional override)
      pub width: Option<u32>,
      pub height: Option<u32>,
  }
  ```
- **D-18:** `id` is a **local per-node identifier** — each machine's config describes its own monitors independently. Phase 8's topology exchange protocol carries these IDs over the wire so peers can correlate edge mappings. There is no cross-node ID collision risk because each machine's config is independent.
- **D-19:** ID matching strategy — **deferred to Phase 8**. Phase 3 just stores the string. Phase 8 implements matching logic against OS-provided identifiers (output name, EDID hash, bus path, etc.). The free-form string design allows matching against any identifier the OS surfaces.
- **D-20:** Phase 5/8 will add `periphore monitors list` to show available OS monitor identifiers so users know what to put in their config.

### Claude's Discretion

- Exact TOML structure for `trusted.toml` (flat list vs `[trusted]` section header)
- Whether `periphore-trust` re-exports via `lib.rs` or uses a `store` module
- Atomic write strategy for `trusted.toml` (write-then-rename vs direct overwrite)
- Whether to add `periphore-trust` to `periphored`'s `Cargo.toml` directly or via `periphore-core`

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` §SEC-05, §SEC-06, §CFG-02, §CFG-03 — the 4 requirements delivered in this phase

### Roadmap
- `.planning/ROADMAP.md` §Phase 3 — success criteria (5 items); note SC4 (topology conflict) is deferred to Phase 8, SC5 (monitor layouts) delivered via MonitorConfig schema

### Prior Phase Decisions
- `.planning/phases/01-workspace-protocol-foundation/01-CONTEXT.md`:
  - D-24: fingerprint cache is separate from main config, NOT owned by periphore-config
  - D-17: IPC socket path pattern (socket path uses `directories` crate)
  - D-03 / D-07: workspace dep declaration pattern (`{ workspace = true }`)
- `.planning/phases/02-identity-cryptography/02-CONTEXT.md`:
  - D-02: XDG data home path via `directories::ProjectDirs` — same pattern for trust cache path
  - D-17: `thiserror`-derived error types in all library crates (TrustError must match this)

### Architecture & Stack
- `.planning/research/ARCHITECTURE.md` — crate responsibility map, build order, channel topology
- `.planning/research/STACK.md` — library selections (serde, figment, directories, thiserror)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/periphore-config/src/schema.rs` — `PeerConfig` already has `fingerprint: Option<String>`, `host`, `port`; add `name: Option<String>`. `TopologyConfig` is empty — Phase 3 populates it with `Vec<MonitorConfig>`.
- `crates/periphore-ipc/src/lib.rs` — `AcceptFingerprint { fingerprint, responder }` and `RejectFingerprint { fingerprint, responder }` IpcCommands already defined. Phase 3 wires real dispatch in `periphored`.
- `crates/periphore-protocol/src/ipc.rs` — `IpcRequest::AcceptFingerprint { fingerprint }` and `IpcRequest::RejectFingerprint { fingerprint }` already defined.
- `crates/periphored/src/main.rs` — `AcceptFingerprint` and `RejectFingerprint` currently fall through to `send_ok()` (stubs returning `IpcResponse::Ok`). Phase 3 promotes them to named arms calling `TrustStore`.

### Established Patterns (must match)
- Library crates use `thiserror` for error types (not `anyhow`) — `TrustError` must follow this
- Daemon and CLI entry points use `anyhow` — trust store errors surface through `anyhow::anyhow!()` at the daemon boundary
- `[lib] test = false` pattern on foundational crates — integration tests in `tests/` subdir
- Workspace deps: `{ workspace = true }` in crate Cargo.tomls; declare in `[workspace.dependencies]` with `path` + `version`
- `directories::ProjectDirs` already in workspace for path resolution — use `data_dir()` for trust cache path (same as key file pattern)

### Integration Points
- `periphored/src/main.rs` AcceptFingerprint arm: load `TrustStore`, call `.add_trusted(fp, alias)`, write to disk, respond `IpcResponse::Ok`
- `periphored/src/main.rs` RejectFingerprint arm: respond `IpcResponse::Ok` (no state change needed — rejection is stateless)
- Phase 6 (TCP peering): import `periphore-trust`, call `.is_trusted(peer_fp)` during handshake; call `check_peer_fingerprint(config_fp, actual_fp, label)` if fingerprint hardcoded in `PeerConfig`

</code_context>

<specifics>
## Specific Ideas

- TOML format for `trusted.toml`: list of `[[trusted]]` entries with `fingerprint` and optional `alias` — mirrors the `[[peer]]` pattern in main config
- Monitor config TOML pattern confirmed by user:
  ```toml
  [[topology.monitor]]
  id = "DP-1"           # OS identifier (xrandr output, CoreGraphics display ID, bus path, etc.)
  name = "primary"      # optional human label
  width = 2560
  height = 1440
  ```
- The `name` field in `[[peer]]` is a local convenience label, NOT part of the peer protocol — documentation must clarify: "fingerprint ultimately identifies the node; name is a local label you assign for your own reference"
- `id` in `[[topology.monitor]]` is local per-node — Phase 8 topology exchange carries these IDs to correlate cross-machine edge mappings; no cross-node uniqueness requirement

</specifics>

<deferred>
## Deferred Ideas

- **VNC/RDP as peers without daemon** — Protocol compatibility mode: Periphore drives input to a VNC/RDP session without requiring the daemon on the remote side. Different transport, no identity handshake. Post-v1 milestone.
- **Topology conflict detection** (SC4: both peers claim the same edge) — Deferred to Phase 8 when actual edge config exists. Phase 3 has no edge definitions to compare.
- **`periphore monitors list` CLI command** — Surfaces available OS monitor identifiers so users know what to put in `id` field. Phase 5/8 concern.
- **Richer TrustStore API** — `.list_trusted() -> &[TrustedPeer]` for `periphore peers list` CLI display. Phase 5 may add this.
- **Trust cache alias in AcceptFingerprint IPC** — The IpcRequest currently only carries `fingerprint: String`. Phase 5 CLI may extend this to carry an optional alias set at accept time.

</deferred>

---

*Phase: 03-configuration-trust-persistence*
*Context gathered: 2026-04-23*
