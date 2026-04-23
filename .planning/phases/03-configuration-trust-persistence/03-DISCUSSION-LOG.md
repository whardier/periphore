# Phase 3: Configuration & Trust Persistence — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-23
**Phase:** 03-configuration-trust-persistence
**Areas discussed:** Trust cache design, Trust store crate placement, Peer alias in config, CFG-03 topology config scope

---

## Trust Cache Design

| Option | Description | Selected |
|--------|-------------|----------|
| XDG data home | `~/.local/share/periphore/trusted.toml` / `~/Library/Application Support/periphore/trusted.toml` | ✓ |
| XDG config home | `~/.config/periphore/trusted.toml` — blurs "never auto-writes" principle | |
| XDG cache home | `~/.cache/periphore/trusted.toml` — cache dirs are conventionally clearable | |

**User's choice:** XDG data home — consistent with key file (Phase 2 D-02), persistent across cache flushes.

---

| Option | Description | Selected |
|--------|-------------|----------|
| TOML | Consistent with main config; serde-parseable; human-readable | ✓ |
| Plain hex list | One fingerprint per line; custom parser required; no metadata | |
| JSON | Consistent with IPC protocol; inconsistent with config layer | |

**User's choice:** TOML — consistent with main config format.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Fingerprint + optional alias | `fingerprint` (required) + `alias` (optional local label) | ✓ |
| Fingerprint only | Minimal; logs refer to peers by hex only | |
| Fingerprint + alias + timestamp | `accepted_at` ISO date; awkward across timezones | |

**User's choice:** Fingerprint + optional alias — alias for human-readable logs.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Only on AcceptFingerprint IPC | Single write path; aligned with single-writer principle | ✓ |
| Phase 3 defines schema only | Defer write dispatch to Phase 6 | |

**User's choice:** Wire AcceptFingerprint IPC dispatch to real trust cache writes in Phase 3.

---

## Trust Store Crate Placement

| Option | Description | Selected |
|--------|-------------|----------|
| New `periphore-trust` crate | Clean separation; 12th crate; between identity and net in build graph | ✓ |
| Extend `periphore-identity` | Simpler; no new crate; identity crate takes on persistence for trust too | |

**User's choice:** New `periphore-trust` crate.
**Notes:** Build order: `protocol → config + identity → trust → core + ipc + cli → net → capture + inject`

---

| Option | Description | Selected |
|--------|-------------|----------|
| `is_trusted` + `add_trusted` (minimal) | `load`, `is_trusted`, `add_trusted`, `remove_trusted` | ✓ |
| Richer API with listing | Add `list_trusted()` now for Phase 5 CLI | |

**User's choice:** Minimal API — Phase 5 adds `list_trusted()` when needed.

---

## Peer Alias in Config

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, optional `name` field | `pub name: Option<String>` in `PeerConfig`; used in logs/errors | ✓ |
| No name field | Peers identified by fingerprint hex or host in all output | |

**User's choice:** Add optional `name` field.
**Notes:** User clarified — `name` is part of the local config experience (accepting and labeling peers), but fingerprint ultimately identifies the node. Documentation must make clear `name` is not exchanged over the wire and has no protocol role.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Error log + refuse connection | `tracing::error!` + `TrustError` returned; Phase 6 drops connection | ✓ |
| Error + IPC event | Emit pending-verification event too; more complex flow | |

**User's choice:** Error log + refuse connection — interactive TOFU flow is a Phase 6 concern.

---

## CFG-03 Topology Config Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Named monitor layout (with `id` + `name`) | `[[topology.monitor]]` with `id`, `name`, `width`, `height` | ✓ |
| Comment placeholder only | Keep `TopologyConfig` empty; defer CFG-03 entirely to Phase 8 | |

**User's choice:** Add `MonitorConfig` schema now to satisfy SC5.
**Notes:** User raised that monitor naming needs an identifier relating to distinct OS identifiers on each node — OS sometimes provides names (xrandr output, CoreGraphics display UUID) but these vary by platform and aren't always stable.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Free-form string `id`, matching deferred | `id: Option<String>` — user puts whatever OS surfaces; Phase 8 defines matching | ✓ |
| Structured ID with type hint | `id_type` + `id` — explicit but adds verbosity without simplifying Phase 8 | |

**User's choice:** Free-form string `id` — Phase 8 owns matching strategy.
**Notes:** User noted `id` must relate to distinct identifiers on each node. Clarified: `id` is local per-node; Phase 8 topology exchange carries these IDs across the wire for edge mapping correlation. No cross-node uniqueness requirement since each machine's config is independent.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Fingerprint mismatch only | SEC-06 conflict detection; topology conflicts deferred to Phase 8 | ✓ |
| Fingerprint + topology conflicts | Speculative; edge config doesn't exist yet in Phase 3 | |

**User's choice:** Fingerprint mismatch only in Phase 3.

---

## Claude's Discretion

- Exact TOML structure for `trusted.toml`
- `periphore-trust` internal module structure
- Atomic write strategy for the cache file
- Whether `periphore-trust` wired through `periphore-core` or directly into `periphored`

## Deferred Ideas

- **VNC/RDP as peers without daemon** — user explicitly requested noting this; protocol compatibility mode for post-v1
- Topology conflict detection — Phase 8
- `periphore monitors list` CLI — Phase 5/8
- `list_trusted()` API — Phase 5
- `alias` in `AcceptFingerprint` IpcRequest — Phase 5 CLI extension
