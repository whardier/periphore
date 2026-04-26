# Phase 6: TCP Peering — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-26
**Phase:** 06-tcp-peering
**Areas discussed:** First-connect verification, Connection initiation, Reconnection on drop, Linux remote launch (NET-05/NET-06)

---

## First-Connect Verification

| Option | Description | Selected |
|--------|-------------|----------|
| Hold in pending | Accept TCP, fingerprint exchange, hold pending state; log at WARN; user runs `periphore trust accept` | ✓ |
| Auto-reject and log | Reject immediately after fingerprint exchange; user manually adds fp to trust cache then reconnects | |
| Pre-trust only | Only accept connections from fingerprints already in trusted.toml | |

**User's choice:** Hold in pending

**Daemon notification method:**

| Option | Description | Selected |
|--------|-------------|----------|
| Daemon logs it | tracing::warn! with fingerprint + identicon + word-phrase to stderr | ✓ |
| Explicit CLI poll command | `periphore peers pending` lists pending connections | |
| Both log + CLI command | WARN log + structured CLI query | |

**User's choice:** Daemon logs it (WARN level)

**Notes:** `GetPendingVerifications` IPC command already defined in periphore-protocol (Phase 1 D-15) — Phase 6 wires the real implementation.

---

## Connection Initiation

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-connect on startup | Daemon auto-connects to all [[peer]] entries with host set; exponential backoff retry | ✓ |
| Manual connect only | Daemon only listens; user runs `periphore connect` to initiate | |
| Both: auto + CLI ad-hoc | Auto-connect on startup + CLI command for ad-hoc | |

**User's choice:** Auto-connect on startup

**Listening behavior:**

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, always listen | Binds TCP port on startup regardless of peer config | |
| Only if peers configured | Binds TCP only when [[peer]] entries exist | |
| Configurable via daemon.listen | New config field, default true; false for CI/testing setups | ✓ |

**User's choice:** Configurable via daemon.listen (new config field)

---

## Reconnection on Drop

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-reconnect with backoff | Exponential backoff: 1s→2s→4s→8s→16s→capped at 30s; INFO log per retry | ✓ |
| No auto-reconnect | Log disconnect, stop; manual reconnect required | |
| Configurable per-peer | auto_reconnect in [[peer]] config | |

**User's choice:** Auto-reconnect with exponential backoff

---

## Linux Remote Launch (NET-05) / macOS Error (NET-06)

**Linux daemonization:**

| Option | Description | Selected |
|--------|-------------|----------|
| Document nohup/systemd | No daemon flag; docs cover nohup + systemd user unit; ship sample .service file | ✓ |
| --daemonize flag | periphored --daemonize double-forks and detaches | |
| Systemd unit only | Only document systemd path | |

**User's choice:** Document nohup/systemd (ship `contrib/periphored.service`)

**macOS remote launch error:**

| Option | Description | Selected |
|--------|-------------|----------|
| Clear error + explanation | macOS SSH detected via isatty(0); print clear error and exit | ✓ |
| Silent fail gracefully | Generic error from failed bind | |
| Platform detection + error | #[cfg(macos)] + isatty check at startup | |

**User's choice:** Clear error + explanation (with specific message text)

---

## Claude's Discretion

- Default TCP port value (avoid 24800/Synergy; pick from 7700–8000 range)
- Internal representation of pending connections
- Exact backoff implementation (tokio-retry vs manual sleep loop)
- `periphore-net` API surface (ConnectionManager struct vs flat async functions)
- `NetError` thiserror design
- `FocusStateMachine` wiring into periphored

## Deferred Ideas

- `periphore connect <host>` ad-hoc CLI command — deferred to Phase 7+
- `periphore peers list` / `periphore peers pending` — deferred to Phase 7
- Hot-reload peer list (add new peers without restart) — future
- Mutual TLS — post-v1
- Connection rate limiting — post-v1
