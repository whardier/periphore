# Phase 1: Workspace & Protocol Foundation — Context

**Gathered:** 2026-04-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 1 delivers a buildable Cargo workspace with:
1. All 9 crate stubs established (workspace dep graph complete from day one)
2. Full protocol type surface defined in `periphore-protocol`
3. Layered config loading in `periphore-config` (full schema, never writes)
4. Full IPC test harness in `periphore-ipc` (Unix domain socket + complete IpcRequest enum)
5. Main daemon binary (`periphore`) starts, creates IPC socket, responds to basic commands
6. `periphore-ctl` scaffolded as a stub (real implementation in Phase 5)

**Scope expansion from roadmap:** IPC implementation is pulled into Phase 1 (not Phase 4). Phase 4 is retained for IPC enhancements only.

New Phase 1 success criteria (supersedes roadmap):
1. `cargo build --workspace` succeeds with all 9 crates present
2. Protocol crate defines the full PeerMessage enum (~15 variants) plus all supporting types — round-trips via `postcard`
3. Config crate loads full schema from TOML with layered precedence (defaults < file < env < CLI), never writes to disk
4. Daemon binary starts, creates Unix domain socket at platform-appropriate path, responds to `GetStatus` IPC command
5. Full IpcRequest enum compiles and is reachable over the socket
6. `periphore-ctl` stub builds with `--help` output

</domain>

<decisions>
## Implementation Decisions

### Workspace Scaffold
- **D-01:** All 9 crates scaffolded in Phase 1 — `crates/` flat layout following uv/typst pattern (`members = ["crates/*"]`)
- **D-02:** `default-members = ["crates/periphore"]` so plain `cargo build` builds the daemon only
- **D-03:** Every crate declared in `[workspace.dependencies]` with both `path` and `version` — no bare path refs inside individual crate Cargo.tomls
- **D-04:** `[workspace.lints.rust]` + `[workspace.lints.clippy]` set from day one; all crates use `[lints] workspace = true`
- **D-05:** Binary crates live inside `crates/`, not at workspace root — `crates/periphore/src/main.rs` and `crates/periphore-ctl/src/main.rs`
- **D-06:** Feature gating via features on internal crates (e.g., `periphore-config` gets a `clap` feature activated only by `periphore-ctl`)
- **D-07:** `[lib] doctest = false test = false` on thin foundational crates (`periphore-protocol`, `periphore-identity`)

### Crates Implemented vs Stubbed
- **D-08:** Actively implemented in Phase 1: `periphore-protocol`, `periphore-config`, `periphore-ipc`, `crates/periphore` (daemon binary)
- **D-09:** Stubbed (empty `src/lib.rs`) for later phases: `periphore-identity` (Phase 2), `periphore-core` (Phase 4+), `periphore-net` (Phase 6), `periphore-capture` (Phase 9), `periphore-inject` (Phase 9)
- **D-10:** `crates/periphore-ctl` scaffolded with thin `src/main.rs` stub — full implementation in Phase 5

### Protocol Types (`periphore-protocol`)
- **D-11:** Full PeerMessage enum defined — all ~15 variants: Hello, HelloAck, TopologyAdvertise, TopologyPropose, TopologyAccept, TopologyReject, FocusTransfer, FocusAck, FocusReclaim, MouseMove, MouseButton, MouseScroll, KeyEvent, Ping, Pong, Bye
- **D-12:** Full supporting type surface defined: `MonitorInfo`, `Edge` (Left/Right/Top/Bottom), `EdgeMapping`, `InputEvent` (Mouse/Key), `MouseEvent`, `KeyEvent`
- **D-13:** Serialization: `serde` + `postcard`; framing: `tokio-util LengthDelimitedCodec` (4-byte big-endian length header)
- **D-14:** Round-trip tests for the full PeerMessage enum in the protocol crate

### IPC Layer (`periphore-ipc`)
- **D-15:** Full IpcRequest enum implemented: `GetStatus`, `ListPeers`, `GetTopology`, `AcceptFingerprint`, `RejectFingerprint`, `ReloadConfig`, `InjectInputEvent { event: InputEvent }`, `SimulateEdgeCross { edge: Edge, position: f64 }`, `GetState`, `GetPendingVerifications`, `GetIdenticon`, `GetWordPhrase`
- **D-16:** Protocol: JSON-lines over Unix domain socket (newline-delimited JSON) — local-only so human-readable is fine
- **D-17:** Socket path: Linux `$XDG_RUNTIME_DIR/periphore/periphore.sock`, macOS `$TMPDIR/periphore/periphore.sock`; permissions `0600`
- **D-18:** Daemon creates socket on startup, removes on clean shutdown (including SIGTERM/SIGHUP via `tokio::signal`)
- **D-19:** IPC implemented as the testing backbone — `InjectInputEvent` and `SimulateEdgeCross` must be exercisable from Phase 1 forward
- **D-20:** Phase 4 (IPC Layer) is retained in the roadmap for IPC enhancements, not removed

### Config Schema (`periphore-config`)
- **D-21:** Full schema defined upfront — all top-level sections present: `[daemon]`, `[logging]`, `[[peer]]`, `[topology]`, `[capture]`
- **D-22:** Layering order: `Figment::new().merge(defaults).merge(toml).merge(env).merge(cli)` — Figment's default order is inverted from what's needed, must be explicit
- **D-23:** Config never writes to disk under any code path — enforced at compile time (no `Serialize` impl on the config struct, no write paths)
- **D-24:** Fingerprint cache is separate from main config (stored in XDG cache dir, written only by trust acceptance flow) — config crate does not own this path
- **D-25:** `clap` feature on `periphore-config` gates the CLI-integrated config struct (only activated by `periphore` and `periphore-ctl`)

### Daemon Binary (`crates/periphore`)
- **D-26:** Thin `src/main.rs` — all command logic in `src/lib.rs`, binary just calls `periphore::main()`
- **D-27:** Daemon starts IPC socket, responds to `GetStatus` with identity fingerprint placeholder and running status
- **D-28:** `--help` produces usage output via `clap` v4 derive API
- **D-29:** Signal handling: SIGTERM and SIGHUP handled via `tokio::signal`; clean shutdown removes IPC socket

### Claude's Discretion
- Exact Clippy lint configuration (which pedantic lints to allow vs warn) — follow uv's pattern of `pedantic = warn` with selective overrides
- Whether to add `resolver = "2"` and `edition = "2024"` at workspace level (yes — both reference projects use this)
- Workspace package metadata fields (`homepage`, `repository`, `license`) — reasonable defaults are fine
- Whether `periphore-protocol` re-exports its types from a top-level `lib.rs` or uses modules — Claude decides

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Architecture & Crate Design
- `.planning/research/ARCHITECTURE.md` — crate workspace structure, channel topology, wire protocol design, IPC design, focus token model, anti-patterns to avoid
- `.planning/research/WORKSPACE-PATTERNS.md` — uv/typst workspace patterns: workspace deps declaration, lints setup, binary crate placement, feature gating, platform crate patterns

### Stack & Library Choices
- `.planning/research/STACK.md` — library selections with rationale and confidence: tokio, tokio-util, postcard, serde, figment, clap v4, directories crate for socket paths

### Requirements
- `.planning/REQUIREMENTS.md` — CFG-01 (config never auto-writes), IPC-01, IPC-02
- `.planning/ROADMAP.md` — Phase 1 success criteria (NOTE: success criteria expanded per CONTEXT.md D-08 through D-20)

### Pitfalls
- `.planning/research/PITFALLS.md` — implementation landmines to check before coding

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- None yet — greenfield project. No existing Rust code.

### Established Patterns
- `prek.toml` — pre-commit hook configuration (conventional commits enforced; `cargo test` and `cargo clippy` likely configured)
- `cz.toml` — commitizen configuration (conventional commit format enforced)

### Integration Points
- All later phases depend on `periphore-protocol` types being stable from Phase 1
- IPC test harness (`InjectInputEvent`, `SimulateEdgeCross`) is the primary testing mechanism for Phases 4–10
- `periphore-config` full schema means later phases only add fields, never restructure

</code_context>

<specifics>
## Specific Ideas

- Follow uv's pattern for internal crate deps: declare in `[workspace.dependencies]` with `path` + `version`, reference as `{ workspace = true }` everywhere — verified against two production Rust multi-crate projects
- Workspace lints: enable `unsafe_code = "warn"` and `unreachable_pub = "warn"` at workspace level from day one
- IPC socket is the test backbone — by Phase 1's end, a test can inject a MouseMove via IPC and observe state change, proving the modular boundary works without a network peer
- Binary crates in `crates/periphore/` and `crates/periphore-ctl/` — not at workspace root (confirmed by uv + typst reference)

</specifics>

<deferred>
## Deferred Ideas

- Full IPC command richness — additional IPC commands beyond the Phase 1 set belong in Phase 4 enhancements
- `periphore-ctl` real implementation — Phase 5
- Identity/fingerprint types in protocol — Phase 2 fills these in (stubs in Phase 1)
- Platform-specific capture/inject — Phases 9–10

</deferred>

---

*Phase: 01-workspace-protocol-foundation*
*Context gathered: 2026-04-22*
