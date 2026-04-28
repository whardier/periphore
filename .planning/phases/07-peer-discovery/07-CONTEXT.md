# Phase 7: Peer Discovery — Context

**Gathered:** 2026-04-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 7 adds mDNS-based peer discovery so daemons on the same local network can find each other automatically:

1. A daemon with `[discovery] enabled = true` broadcasts its presence via `_periphore._tcp.local.`
2. Other daemons listen for these announcements and maintain a passive in-memory list of discovered peers
3. Users inspect the list via CLI (`periphore peers discovered`), then add interesting peers to their config for connection
4. When mDNS fails silently (corporate network, firewall), the daemon logs a warning and manual `[[peer]] host=` config continues to work as the primary path

**Out of scope for Phase 7:**
- Auto-connecting to discovered peers (user adds config + restarts — see D-09)
- `periphore connect <host>` on-demand connect command (deferred)
- `periphore peers list` combined view (deferred)
- Monitor topology exchange (Phase 8)
- Input event forwarding (Phase 9)

</domain>

<decisions>
## Implementation Decisions

### Crate Placement

- **D-01:** mDNS discovery logic lives in a **new `periphore-discovery` crate** at `crates/periphore-discovery`. Not inside `periphore-net` or `periphored` directly.
- **D-02:** `periphore-discovery` depends on `periphore-net` + `periphore-config`. Build order: after `periphore-net`, before `periphored`. Add `periphore-discovery` to workspace `Cargo.toml` `[workspace.dependencies]` and to `periphored`'s `[dependencies]`.

### Discovery Config

- **D-03:** mDNS discovery is **opt-in** — disabled by default. User enables by adding `[discovery]\nenabled = true` to their TOML config. Consistent with CFG-01 (config is user-authored; no auto-enabling surprises on corporate/restricted networks).
- **D-04:** New top-level `[discovery]` section in `periphore-config` `schema.rs` as a `DiscoveryConfig` struct with fields:
  - `enabled: bool` — default `false`
  - `instance_name: Option<String>` — optional override for the mDNS service instance name (defaults to the local hostname)
  - `service_type: String` — default `_periphore._tcp.local.`

### Auto-Connect Behavior

- **D-05:** Discovered peers are **passive** — tracked in memory but NOT auto-connected. No TCP connection is initiated on discovery. User must add `[[peer]]\nhost = "<hostname>"` to config and restart to connect.
- **D-06:** New `IpcRequest::GetDiscoveredPeers` variant in `periphore-protocol/src/ipc.rs`. Response: `IpcResponse::DiscoveredPeers(Vec<DiscoveredPeerInfo>)`. `DiscoveredPeerInfo` contains: `hostname: String`, `port: u16`, `last_seen: std::time::SystemTime` (or serializable timestamp). Add both types adjacent to `PendingPeerInfo` in `ipc.rs`.
- **D-07:** Discovered list uses **hybrid expiry**: remove entry immediately when mDNS goodbye event fires (primary); also expire entries via TTL garbage collection (safety net for networks where goodbye packets are lost).
- **D-08:** TTL for garbage-collected entries: **5 minutes** since `last_seen`. A background task sweeps the list periodically; each mDNS re-announcement refreshes the `last_seen` timestamp.
- **D-09:** Discovered list is capped at **64 peers**. When the cap is hit: evict the entry with the oldest `last_seen` timestamp. Log at `tracing::warn!` when eviction occurs due to cap overflow.

### CLI Additions

- **D-10:** `periphore peers discovered` — new subcommand in `periphore-cli`. Sends `IpcRequest::GetDiscoveredPeers`, displays result as a table: hostname/IP, port, last-seen time. Shows a helpful note if the list is empty ("No peers discovered. Enable discovery in config: [discovery] enabled = true").
- **D-11:** `periphore peers pending` — new subcommand in `periphore-cli`. Sends `IpcRequest::GetPendingVerifications` (already defined in Phase 6, just needs wiring in CLI). Displays pending peers: fingerprint hex + word phrase for out-of-band verification.

### Claude's Discretion

- Exact mDNS TXT record fields broadcast alongside the service announcement (e.g., protocol version, port override — use minimal fields; hostname + port is sufficient for discovery)
- Internal struct representation of `DiscoveredPeerInfo` (serialization format for IPC)
- Whether `periphore-discovery` exposes a channel-based API (`mpsc::Receiver<DiscoveryEvent>`) or a callback/task-based approach — use channel-based to match `PeerEvent` pattern from `periphore-net`
- Exact sweep interval for the TTL GC task (30s sweep, 5-minute TTL is the expiry threshold)
- Output table format for `periphore peers discovered` (align with `periphore status` output style)
- Error type design for `periphore-discovery` (`thiserror`-derived `DiscoveryError`)
- Whether the mDNS service instance name includes a random suffix to avoid collisions on multi-daemon hosts
- `periphore connect <host>` on-demand connect command — deferred to a future phase, not in Phase 7
- `periphore peers list` combined view (active + pending + discovered) — deferred to a future phase

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/REQUIREMENTS.md` §NET-02 — the single requirement this phase delivers
- `.planning/ROADMAP.md` §Phase 7 — 3 success criteria, Depends on Phase 6

### Protocol (IPC additions)
- `crates/periphore-protocol/src/ipc.rs` — `IpcRequest`, `IpcResponse`, `PendingPeerInfo` (model `DiscoveredPeerInfo` after `PendingPeerInfo`; add `GetDiscoveredPeers` variant adjacent to `GetPendingVerifications`)

### Config (schema additions)
- `crates/periphore-config/src/schema.rs` — existing `Config` struct (add `discovery: DiscoveryConfig` field with `#[serde(default)]`); existing `DaemonConfig` (for port reference); existing `PeerConfig` (manual host pattern that discovery helps populate)

### Connection layer (integration target)
- `crates/periphore-net/src/manager.rs` — `ConnectionManager` API; `periphore-discovery` triggers connects into this manager for the future connect-on-demand path; passive list in Phase 7 does NOT call into ConnectionManager
- `crates/periphore-net/src/lib.rs` — exported types (`DEFAULT_PORT`) for broadcast port
- `.planning/phases/06-tcp-peering/06-CONTEXT.md` §D-01 to §D-11 — trust/pending flow that discovered peers will eventually feed into; also D-07 (symmetric listen) and D-08 (port 7888)

### CLI layer
- `crates/periphore-cli/src/lib.rs` — CLI dispatch and existing subcommand patterns
- `crates/periphore-cli/src/commands/` — existing `status.rs` and `topology.rs` for pattern reference when adding `peers/discovered.rs` and `peers/pending.rs`
- `crates/periphore/src/main.rs` — CLI entry point wiring

### Daemon wiring
- `crates/periphored/src/main.rs` — existing select! loop pattern; Phase 7 adds discovery crate spawn and `IpcCommand::GetDiscoveredPeers` dispatch

### Critical notes (CLAUDE.md)
- `CLAUDE.md` §Discovery — `mdns-sd` is the specified discovery crate
- `CLAUDE.md` §Critical Implementation Notes — mDNS fails silently on corporate networks (item 6 context); manual host config must always work as fallback

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/periphore-protocol/src/ipc.rs` — `PendingPeerInfo { fingerprint_hex: String, addr: String, word_phrase: Vec<String> }` — template for new `DiscoveredPeerInfo` struct; `IpcResponse::PendingPeers(Vec<PendingPeerInfo>)` — template for `IpcResponse::DiscoveredPeers`
- `crates/periphore-net/src/manager.rs` — `ConnectionManager` with `spawn_listener()` and connector loop — Phase 7 creates `periphore-discovery`'s `DiscoveryService` following the same spawn-into-JoinSet pattern
- `crates/periphored/src/main.rs` — JoinSet + CancellationToken + select! pattern; `periphore-discovery` spawns into the same JoinSet

### Established Patterns
- `thiserror`-derived error enums in library crates — `DiscoveryError` must follow this
- `[lib] test = false` + integration tests in `tests/` subdir — `periphore-discovery` must follow
- Workspace deps: `{ workspace = true }` in crate Cargo.tomls — add `mdns-sd` to `[workspace.dependencies]` first
- `#[serde(default)]` on config structs + manual `Default` impl — `DiscoveryConfig::default()` returns `enabled: false`
- `tracing::warn!` for mDNS failure notifications (silent network failure → warn, not error)
- oneshot responder pattern for IPC: `IpcCommand::GetDiscoveredPeers(oneshot::Sender<IpcResponse>)` — matches existing variants

### Integration Points
- `crates/periphore-config/src/schema.rs` → add `discovery: DiscoveryConfig` to `Config` struct
- `crates/periphore-protocol/src/ipc.rs` → add `GetDiscoveredPeers`, `DiscoveredPeerInfo`, `IpcResponse::DiscoveredPeers`
- `crates/periphored/src/main.rs` → wire `periphore-discovery::DiscoveryService`, add `IpcCommand::GetDiscoveredPeers` dispatch arm, extend `send_ok!` exhaustive match
- `crates/periphore-cli/src/lib.rs` → add `peers` subcommand group with `discovered` and `pending` sub-subcommands

</code_context>

<specifics>
## Specific Ideas

- The discovered list table output from `periphore peers discovered` should include a hint when empty: "No peers discovered — make sure discovery is enabled in your config ([discovery] enabled = true) and both machines are on the same subnet."
- Discovery failure on bind (mDNS socket bind fails) should log `tracing::warn!` and continue — daemon starts normally, manual `[[peer]]` config is the fallback. Do NOT fail daemon startup on discovery bind failure.
- mDNS service instance name should default to the system hostname; users can override with `instance_name` in `[discovery]` config for clarity on shared networks.

</specifics>

<deferred>
## Deferred Ideas

- `periphore connect <host>` on-demand connect command — deferred to Phase 8 or later once peer management matures
- `periphore peers list` combined view (active + pending + discovered) — deferred to future phase
- Auto-connect on discovery (daemon connects immediately to discovered peers without user action) — user explicitly chose passive list model; reconsider post-v1
- TXT record fingerprint advertisement (broadcasting fingerprint hex in mDNS TXT so users can pre-verify before connecting) — potentially useful, deferred from Phase 7 scope

</deferred>

---

*Phase: 07-peer-discovery*
*Context gathered: 2026-04-28*
