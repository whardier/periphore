# Phase 7: Peer Discovery - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-28
**Phase:** 07-peer-discovery
**Areas discussed:** Crate placement, Discovery config, Auto-connect behavior, CLI scope

---

## Crate Placement

| Option | Description | Selected |
|--------|-------------|----------|
| Extend periphore-net | Add discovery module inside periphore-net alongside manager.rs — simpler, no new crate | |
| New periphore-discovery crate | Separate crate with mdns-sd, depends on periphore-net + periphore-config | ✓ |
| Inside periphored directly | Discovery task spawned in periphored/src/main.rs | |

**User's choice:** New `periphore-discovery` crate
**Notes:** Cleaner separation of concerns. Depends on periphore-net + periphore-config (to read port and trigger connections). Build order: after periphore-net, before periphored.

---

## Discovery Config

| Question | Option | Selected |
|----------|--------|----------|
| Default behavior | Opt-in (disabled by default) | ✓ |
| | Opt-out (enabled by default) | |
| Config structure | New `[discovery]` section | ✓ |
| | Field on `[daemon]` | |
| Service type | `_periphore._tcp.local.` | ✓ |
| | You decide | |

**User's choices:**
- Opt-in: user must explicitly set `[discovery] enabled = true`
- New top-level `[discovery]` section with `enabled`, `instance_name`, `service_type` fields
- Service type: `_periphore._tcp.local.`

**Notes:** Consistent with CFG-01 (config is user-authored). Opt-in protects users on corporate/restricted networks from unexpected mDNS traffic.

---

## Auto-Connect Behavior

| Question | Option | Selected |
|----------|--------|----------|
| On discovery | Auto-connect immediately (→ Pending/trust flow) | |
| | Passive list only — user manually connects | ✓ |
| List surfacing | IpcRequest::GetDiscoveredPeers | ✓ |
| | In-memory only, user uses logs | |
| Goodbye handling | Remove on mDNS goodbye | original selection |
| | TTL-based expiry | |
| Hybrid | Both goodbye + TTL | ✓ (user correction) |
| TTL value | 5 minutes | ✓ |
| | 30 seconds | |
| | You decide | |
| List cap | 64 peers | ✓ |
| | 256 peers | |
| Connect mechanism | periphore connect <host> | |
| | Add [[peer]] host= to config + restart | ✓ |

**User's choices:**
- Passive list model — discovered peers are NOT auto-connected
- `IpcRequest::GetDiscoveredPeers` IPC command exposes the list
- Hybrid expiry: mDNS goodbye event (primary) + 5-minute TTL GC (safety net)
- 64-peer cap with oldest-seen eviction on overflow
- To connect: user adds `[[peer]] host=<hostname>` to config and restarts

**Notes:** User noted the need for TTL + GC explicitly to avoid DoS: "we also need the daemon to use a ttl as well.. to garbage collect.. and we should only keep the top (reasonable amount) in memory to avoid a denial of service."

---

## CLI Scope

| Option | Description | Selected |
|--------|-------------|----------|
| periphore peers discovered | Show mDNS-discovered peers: hostname/IP, port, last-seen | ✓ |
| periphore connect \<host\> | On-demand connect (Phase 6 deferred) | |
| periphore peers list | All active/pending connections combined | |
| periphore peers pending | Peers awaiting trust acceptance | ✓ |

**Clarification:** Since `periphore connect <host>` was not selected, user confirmed: "Add [[peer]] host= to config + restart" is the connect workflow in Phase 7.

**periphore peers discovered output:** hostname/IP, port, last-seen time (recommended choice)

**Notes:** Phase 7 adds a `peers` subcommand group with two sub-subcommands (`discovered` and `pending`). `periphore connect` and `periphore peers list` remain deferred.

---

## Claude's Discretion

- Exact mDNS TXT record fields (hostname + port is sufficient; protocol version optional)
- Internal `DiscoveredPeerInfo` struct fields
- Channel-based vs callback API for `periphore-discovery` (Claude to use channel, matching `PeerEvent` pattern)
- Sweep interval for TTL GC task (30s sweep cadence)
- `periphore peers discovered` output table format

## Deferred Ideas

- `periphore connect <host>` — explicitly deferred by user; future phase
- `periphore peers list` — explicitly deferred by user; future phase
- Auto-connect on discovery — user chose passive model; reconsider post-v1
- TXT record fingerprint broadcast — potentially useful; out of Phase 7 scope
