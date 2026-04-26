# Phase 6: TCP Peering — Context

**Gathered:** 2026-04-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 6 establishes the TCP peering layer between two Periphore daemons:

1. Each daemon listens for incoming peer TCP connections (configurable via `daemon.listen`)
2. Daemon auto-connects to all `[[peer]]` entries with `host` set on startup, with exponential backoff retry
3. Peers complete a verified identity handshake (Hello/HelloAck + fingerprint exchange)
4. Unknown peers are held in a "pending" state — input forwarding blocked until user accepts
5. Dropped connections auto-reconnect with exponential backoff
6. Linux: daemon runs as a foreground process, daemonization delegated to nohup/systemd
7. macOS: remote SSH launch produces a clear error; daemon must be pre-running locally

**Out of scope for Phase 6:**
- mDNS peer discovery (Phase 7)
- Monitor topology exchange (Phase 8)
- Input event forwarding (Phase 9)
- CLI command for ad-hoc `periphore connect <host>` (Phase 7 or later, once discovery exists)

</domain>

<decisions>
## Implementation Decisions

### First-Connect Verification Flow

- **D-01:** Unknown peer behavior: **hold in pending**. The daemon accepts the TCP connection, completes fingerprint exchange (Hello/HelloAck), then holds the connection in a `Pending` state. Input forwarding is blocked until the user accepts.
- **D-02:** Pending notification: `tracing::warn!` at WARN level with the peer's fingerprint hex, identicon, and word-phrase printed to stderr/logs. User sees it in daemon output, then runs `periphore trust accept <fp>` to promote to trusted.
- **D-03:** The `GetPendingVerifications` IPC command (already defined in `periphore-protocol`) surfaces pending connections for CLI/tooling use. Phase 6 must wire its real implementation (currently a stub).
- **D-04:** On fingerprint conflict (known peer presents wrong fingerprint): `tracing::error!` + drop connection — **locked Phase 3 D-14**. This is already specified by `check_peer_fingerprint()` in `periphore-trust`.

### Connection Initiation

- **D-05:** Daemon **auto-connects on startup** to all `[[peer]]` entries that have `host` set. No manual CLI initiation command needed in Phase 6.
- **D-06:** Auto-connect uses exponential backoff retry in the background. Connection failure is not a daemon startup error — it logs and retries.
- **D-07:** Daemon **listens symmetrically** for incoming connections — either side can initiate (P2P model). Listening is controlled by a new `daemon.listen = true/false` config field (default: `true`). Setting `false` lets CI/testing setups skip TCP binding entirely.
- **D-08:** Default TCP port: defined in `DaemonConfig.port` (already in config schema). Phase 6 picks a concrete default if none is configured. Downstream planner selects the default port value (Claude's discretion — e.g., 7799 or similar that doesn't conflict with Synergy's 24800).

### Reconnection on Disconnect

- **D-09:** When an established peer connection drops unexpectedly: **auto-reconnect with exponential backoff**. Retry schedule: 1s → 2s → 4s → 8s → 16s → capped at 30s. Each retry attempt logs at INFO level.
- **D-10:** Auto-reconnect applies to connections the daemon initiated (outbound to `[[peer]]` entries). For inbound connections that drop, the remote daemon handles the reconnect outbound (symmetric model).
- **D-11:** If a peer is removed from config, its auto-reconnect loop must be cancelled. Config reload (already implemented via SIGHUP/ReloadConfig, Phase 4) should diff the peer list and cancel orphaned reconnect tasks.

### Linux Remote Launch (NET-05)

- **D-12:** Daemon runs as a **single foreground process**. No `--daemonize` flag, no double-fork. Daemonization is the operator's responsibility.
- **D-13:** Documentation covers two paths for persistent operation:
  - Quick: `nohup periphored &` with stdout/stderr redirected to a log file
  - Recommended: systemd user unit (`~/.config/systemd/user/periphored.service`)
- **D-14:** A sample `periphored.service` systemd user unit file is shipped in the repository (e.g., `contrib/periphored.service`). Phase 6 creates this file.

### macOS Remote Launch Error (NET-06)

- **D-15:** At startup on macOS, if the daemon detects it is running non-interactively over SSH (stdin is not a TTY: `!isatty(0)`), it prints a clear error to stderr and exits:
  ```
  error: periphored must be launched from a local terminal or launchd on macOS.
         Remote SSH launch is not supported on macOS.
         Start the daemon locally, then connect to it via SSH tunnel if needed.
  ```
- **D-16:** This check is macOS-only (`#[cfg(target_os = "macos")]`). Linux ignores it — SSH launch is the valid path on Linux.

### periphore-net Crate Implementation

- **D-17:** `periphore-net` is currently a 2-line stub. Phase 6 is its primary implementation phase.
- **D-18:** Framing: `tokio-util::codec::LengthDelimitedCodec` (4-byte big-endian length header) + `postcard` serialization — **locked Phase 1 D-13**. No deviation.
- **D-19:** `TCP_NODELAY` must be set **immediately** after `TcpStream::connect()` / after `TcpListener::accept()` — **CLAUDE.md hard requirement**.
- **D-20:** `periphore-net` depends on: `periphore-protocol`, `periphore-identity`, `periphore-trust`, `periphore-config`. Build order maintained from Phase 3 D-06.
- **D-21:** `periphored` adds `periphore-net` as a dependency in Phase 6 and wires `periphore-core`'s `FocusStateMachine` (Phase 4 D-10 deferred this to Phase 6).

### Claude's Discretion

- Default TCP port value when `daemon.port` is not configured (avoid 24800/Synergy; pick an unused port in the 7700–8000 range)
- Internal representation of "pending" connections (e.g., `HashMap<PeerId, PendingPeer>` in a connection manager struct)
- Exact exponential backoff implementation (tokio-retry crate vs manual tokio::time::sleep loop)
- Whether `periphore-net` exposes a `ConnectionManager` struct or a flat async function API
- Error type design for `periphore-net` (`thiserror`-derived `NetError` — consistent with other library crates)
- Exact wiring of `periphore-core::FocusStateMachine` into `periphored` — Phase 4 deferred this; Phase 6 adds the dep and routes `SimulateEdgeCross` through it

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Protocol & Framing
- `crates/periphore-protocol/src/peer.rs` — `PeerMessage` enum: `Hello`, `HelloAck`, `Bye`, and all other wire message variants
- `crates/periphore-protocol/src/ipc.rs` — `IpcRequest::GetPendingVerifications` (must be wired to real implementation in Phase 6)
- `.planning/phases/01-workspace-protocol-foundation/01-CONTEXT.md` §D-13 — framing: LengthDelimitedCodec (4-byte big-endian) + postcard (LOCKED)

### Trust & Fingerprint
- `crates/periphore-trust/src/lib.rs` — `TrustStore::is_trusted()`, `check_peer_fingerprint()` — Phase 6 calls these during handshake
- `.planning/phases/03-configuration-trust-persistence/03-CONTEXT.md` §D-13, §D-14 — fingerprint conflict behavior: tracing::error! + drop (LOCKED)

### Config Schema
- `crates/periphore-config/src/schema.rs` — `DaemonConfig.port`, `DaemonConfig.listen` (new field, Phase 6 adds it), `PeerConfig.host`, `PeerConfig.port`, `PeerConfig.fingerprint`

### State Machine
- `crates/periphore-core/src/lib.rs` — `FocusStateMachine`, `FocusState`, `PeerId` — Phase 6 wires this into periphored
- `.planning/phases/04-ipc-layer/04-CONTEXT.md` §D-10 — FocusStateMachine integration deferred to Phase 6 (explicitly)

### Requirements
- `.planning/REQUIREMENTS.md` §NET-01, §NET-03, §NET-04, §NET-05, §NET-06 — all 5 requirements delivered in this phase
- `.planning/ROADMAP.md` §Phase 6 — 5 success criteria

### Critical Implementation Notes (CLAUDE.md)
- `CLAUDE.md` §Critical Implementation Notes — TCP_NODELAY (item 1), macOS Secure Input (item 2), modifier desync on edge crossing (item 5), mouse-move coalescing (item 6)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/periphore-protocol/src/peer.rs` — `PeerMessage` enum fully defined; `Hello { protocol_version, fingerprint: [u8; 32], public_key: Vec<u8> }` and `HelloAck { fingerprint, public_key, accepted }` are the handshake messages. Phase 6 implements the actual TCP framing around these.
- `crates/periphore-trust/src/lib.rs` — `TrustStore`, `check_peer_fingerprint()` ready to use; Phase 6 calls `.is_trusted()` and `check_peer_fingerprint()` during handshake
- `crates/periphore-identity/src/lib.rs` — `IdentityStore` with keypair and fingerprint; Phase 6 reads the local fingerprint to send in `Hello`
- `crates/periphore-core/src/lib.rs` — `FocusStateMachine` with `transfer_to()` and `reclaim()` — Phase 6 wires this into periphored's connection loop
- `crates/periphore-ipc/src/lib.rs` — `IpcCommand::GetPendingVerifications` defined (stub dispatch); Phase 6 promotes to real implementation
- `crates/periphore-config/src/schema.rs` — `DaemonConfig.port`, `PeerConfig.host/port/fingerprint/name` all present; Phase 6 adds `DaemonConfig.listen`

### Established Patterns
- `thiserror`-derived error enums in library crates — `NetError` must follow this pattern
- `anyhow` at daemon boundary (`periphored/src/main.rs`) — net errors surface via `anyhow::anyhow!()` at the wiring layer
- `[lib] test = false` + integration tests in `tests/` subdir — `periphore-net` must follow this
- Workspace deps: `{ workspace = true }` in crate Cargo.tomls; declare in `[workspace.dependencies]` with `path` + `version`
- `tracing::warn!` / `tracing::error!` / `tracing::info!` for runtime events
- Atomic write via `tempfile::NamedTempFile` + `persist()` rename (used in trust store) — if any Phase 6 writes go to disk, use this pattern
- IpcCommand oneshot responder pattern: each variant carries `oneshot::Sender<IpcResponse>` — `GetPendingVerifications` follows this

### Integration Points
- `periphored/src/main.rs` — Phase 6 adds `periphore-net` and `periphore-core` deps; wires connection manager into the main select! loop
- `periphore-net` will need channels for: incoming peer events → periphored, IPC commands → peer connections (e.g., AcceptFingerprint promoting a pending peer)
- `AcceptFingerprint` IPC dispatch in `periphored`: currently calls `trust_store.add_trusted()`. Phase 6 must also promote the pending connection to active after trust is added.
- `GetPendingVerifications` IPC dispatch in `periphored`: currently a stub (`send_ok()`). Phase 6 wires it to return the actual pending peers list.

</code_context>

<specifics>
## Specific Ideas

- Handshake state machine in `periphore-net`: `Connecting` → `Handshaking` → `Pending` (unknown peer) or `Connected` (trusted peer) → `Active` (input forwarding enabled after Phase 9)
- The `Hello` message already has `protocol_version: u32` — Phase 6 should enforce version compatibility (mismatch → `HelloAck { accepted: false }` + disconnect)
- `PeerId` in `periphore-core` is a newtype wrapping fingerprint hex string — Phase 6 aligns this with the fingerprint from `periphore-identity`
- Pending peer display format at WARN level should mirror the identicon output already implemented in Phase 2 (`resolve_identicon()` in periphored) — reuse the same function for consistency

</specifics>

<deferred>
## Deferred Ideas

- `periphore connect <host>` CLI command for ad-hoc connections not in config — deferred to Phase 7 or later once discovery exists
- `periphore peers list` / `periphore peers pending` CLI subcommands — deferred to Phase 7 when peer management becomes richer
- Hot-reload of peer list without restart — Phase 6 config reload diffs peer list and cancels orphaned reconnects; adding new peers requires restart (hot-add deferred)
- Mutual TLS or additional transport-layer encryption — current model: identity verification via Ed25519 fingerprint handshake; TLS is a future concern
- Connection rate limiting / DDoS protection — post-v1

</deferred>

---

*Phase: 06-tcp-peering*
*Context gathered: 2026-04-26*
