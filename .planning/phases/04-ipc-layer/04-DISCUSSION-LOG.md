# Phase 4: IPC Layer — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-24
**Phase:** 04-ipc-layer
**Areas discussed:** Phase 4 scope, Config reload, periphore-core, CR-01 fix

---

## Phase 4 Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Config reload only | SIGHUP + ReloadConfig IPC only; periphore-core stays stub | |
| Config reload + periphore-core | Both config reload and focus state machine in Phase 4 | ✓ |
| Mark complete, advance | IPC-01/IPC-02 are done; fold TODOs into later phases | |

**User's choice:** Config reload + periphore-core
**Notes:** IPC requirements were delivered in Phase 1. Phase 4 is the home for the enhancements that were explicitly deferred there (Phase 1 CONTEXT D-20).

---

## Config Reload

| Option | Description | Selected |
|--------|-------------|----------|
| Safe subset only | Reload logging level + show_identicon only | |
| Full config reload | Re-read all fields; warn on restart-required fields | ✓ |
| Logging level only | Minimal: just reload the tracing filter | |

**User's choice:** Full config reload
**Notes:** Log warnings for fields that require restart. Daemon continues with old values for those. No crash on reload failure.

---

## periphore-core

| Option | Description | Selected |
|--------|-------------|----------|
| Focus/transfer state machine | LocalFocus / ForwardingTo(peer_id) states | ✓ |
| Skeleton only | Public API with todo!() bodies | |
| Defer periphore-core | Implement inline in periphored, extract later | |

**User's choice:** Focus/transfer state machine
**Notes:** Two-state machine: LocalFocus and ForwardingTo { peer_id }. Pure Rust, no async, no platform deps, fully unit-testable.

---

## Core Wiring

| Option | Description | Selected |
|--------|-------------|----------|
| Library only in Phase 4 | Implement with tests; periphored adopts in Phase 6 | ✓ |
| Wire into periphored now | SimulateEdgeCross routes through state machine in Phase 4 | |

**User's choice:** Library only in Phase 4
**Notes:** Phase 6 wires periphore-core in when real peers trigger focus transfers.

---

## Focus States

| Option | Description | Selected |
|--------|-------------|----------|
| Local / Forwarding-to-peer | Two states: LocalFocus, ForwardingTo(peer_id) | ✓ |
| Local / Forwarding / Reclaiming | Three states with explicit reclaim window | |
| Full state machine | All PeerMessage variants affecting focus | |

**User's choice:** Local / Forwarding-to-peer
**Notes:** Simple, clean, covers Phases 6–9 without over-engineering. Reclaiming state can be added in Phase 8/9 if the complexity warrants it.

---

## CR-01 Fix

| Option | Description | Selected |
|--------|-------------|----------|
| Fix CR-01 in Phase 4 | JoinSet guard + clean-exit break | ✓ |
| Defer CR-01 | Leave for Phase 6 when JoinSet gains more tasks | |

**User's choice:** Fix CR-01 in Phase 4
**Notes:** Small, safe fix. Keeps the daemon healthy before Phase 6 adds TCP tasks to the JoinSet.

---

## Claude's Discretion

- Exact tracing-subscriber reload::Layer wiring (EnvFilter vs LevelFilter)
- Whether config reload logs a summary of what changed
- PeerId newtype internal style

## Deferred Ideas

- Wiring periphore-core into periphored — Phase 6
- SimulateEdgeCross routing through FocusStateMachine — Phase 6/8
- Additional focus states (Reclaiming, multi-peer) — Phase 8/9
- Hot-reload of peer list — Phase 6
