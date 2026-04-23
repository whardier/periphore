# Phase 1: Workspace & Protocol Foundation — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-22
**Phase:** 01-workspace-protocol-foundation
**Areas discussed:** Workspace scaffold scope, IPC in Phase 1, Phase 4 fate, IPC scope, Protocol depth, Config schema depth, Binary targets

---

## IPC in Phase 1

| Option | Description | Selected |
|--------|-------------|----------|
| IPC types in protocol only | Define IpcRequest/IpcResponse types alongside PeerMessage, no implementation | |
| IPC crate stubbed | Scaffold periphore-ipc with empty src/lib.rs, implement in Phase 4 | |
| IPC fully implemented | Unix domain socket + full IpcRequest enum working in Phase 1 | ✓ |

**User's choice:** Full IPC implementation in Phase 1
**Notes:** IPC is needed as the testing backbone from day one. Having InjectInputEvent and SimulateEdgeCross working in Phase 1 means all later phases can be tested without a real network peer.

---

## Phase 4 Fate

| Option | Description | Selected |
|--------|-------------|----------|
| Remove Phase 4 | IPC-01 and IPC-02 absorbed into Phase 1; downstream phases renumber | |
| Keep Phase 4 for enhancements | Phase 1 implements core IPC; Phase 4 stays for extensions and richer commands | ✓ |

**User's choice:** Keep Phase 4 for enhancements
**Notes:** Phase 4 is not removed from the roadmap. It will cover IPC enhancements after Phase 1 establishes the foundation.

---

## IPC Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Core socket only | Unix socket + GetStatus + request/response framing proven | |
| Full test harness | Complete IpcRequest enum including InjectInputEvent, SimulateEdgeCross, GetState | ✓ |

**User's choice:** Full test harness
**Notes:** The IpcRequest enum from the research is implemented in full — GetStatus, ListPeers, GetTopology, AcceptFingerprint, RejectFingerprint, ReloadConfig, InjectInputEvent, SimulateEdgeCross, GetState, GetPendingVerifications, GetIdenticon, GetWordPhrase.

---

## Workspace Scaffold Scope

| Option | Description | Selected |
|--------|-------------|----------|
| All 9 stubs | Every crate gets Cargo.toml + src/lib.rs + workspace dep declaration from day one | ✓ |
| Active crates only | Only protocol + config + ipc + periphore binary; others added as phases begin | |

**User's choice:** All 9 stubs
**Notes:** Following uv (67 crates) and typst (16 crates) pattern — both scaffold the full workspace upfront. Workspace dependency graph complete from day one means no Cargo.toml surgery when later phases begin.

---

## Protocol Message Coverage

| Option | Description | Selected |
|--------|-------------|----------|
| Full designed enum | All ~15 PeerMessage variants + all supporting types (MonitorInfo, Edge, EdgeMapping, InputEvent) | ✓ |
| Partial — IPC-required types only | InputEvent, Edge, and types IPC needs; PeerMessage variants added per phase | |

**User's choice:** Full designed enum
**Notes:** IPC test harness requires InputEvent and Edge types from day one, and defining the full PeerMessage enum now means later phases implement logic against stable types rather than adding new variants.

---

## Config Schema Depth

| Option | Description | Selected |
|--------|-------------|----------|
| Full schema | All top-level sections: [daemon], [logging], [[peer]], [topology], [capture] | ✓ |
| Minimal — daemon + logging only | Later phases add their sections as they're implemented | |

**User's choice:** Full schema
**Notes:** Figment layered loading is proven against the real schema. Later phases add fields within their sections but never restructure the schema.

---

## Binary Targets

| Option | Description | Selected |
|--------|-------------|----------|
| All binary crates scaffolded | periphored (daemon) active + periphore (CLI entry) and periphore-cli (library) as stubs | ✓ (implied by "all 11 stubs") |
| Active crates only | periphore and periphore-cli added when Phase 5 begins | |

**User's choice:** All scaffolded — periphored active, periphore + periphore-cli as stubs for Phase 5
**Notes:** Binary crates in `crates/periphored/` (daemon) and `crates/periphore/` (CLI entry); `crates/periphore-cli/` is a library (no main). Full CLI implementation in Phase 5.

---

## Reference Project Research

The user requested research into uv (astral-sh/uv, 67 crates) and typst (typst/typst, 16 crates) workspace structures before answering the scaffold question. Key findings applied:
- Both use flat `crates/` layout with `members = ["crates/*"]`
- Binary crates live inside `crates/`, not at workspace root
- All inter-crate deps declared in `[workspace.dependencies]` with `path` + `version`
- `[workspace.lints]` + `[lints] workspace = true` on every crate
- Feature gating per consumer for optional capabilities (e.g., `clap` feature on config crate)

Findings saved to `.planning/research/WORKSPACE-PATTERNS.md`.

---

## Claude's Discretion

- Exact Clippy lint configuration (which pedantic lints to allow vs warn)
- Workspace package metadata fields (homepage, repository, license)
- Whether periphore-protocol uses module structure or flat lib.rs for type organization
- `resolver = "2"` and `edition = "2024"` at workspace level (implied yes from reference projects)

## Deferred Ideas

- Full IPC command richness beyond Phase 1 set → Phase 4 enhancements
- periphore-cli real implementation → Phase 5
- Identity/fingerprint type details → Phase 2
- Platform-specific capture/inject implementation → Phases 9–10
