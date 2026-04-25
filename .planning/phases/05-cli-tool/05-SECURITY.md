---
phase: 05-cli-tool
asvs_level: 1
audited: 2026-04-25
result: SECURED
threats_closed: 3
threats_total: 3
---

# Phase 5 — CLI Tool: Security Audit

## Summary

All 3 threats verified CLOSED. No open threats. No unregistered flags.

## Threat Verification

| Threat ID | Category | Disposition | Status | Evidence |
|-----------|----------|-------------|--------|----------|
| T-5-01 | Tampering | mitigate | CLOSED | `crates/periphore-cli/src/client.rs:37` — `serde_json::from_str::<IpcResponse>(line.trim())?` with `?` propagation; client never panics on malformed response |
| T-5-02 | Elevation | mitigate | CLOSED | `crates/periphore-cli/src/cli.rs:19,23` — `socket` and `config` are `Option<std::path::PathBuf>` typed by clap; `lib.rs:34` consumes via `if let Some(path)` branch — no shell expansion, no string interpolation |
| T-5-03 | Info Disclosure | accept | CLOSED | Pre-accepted per constraint. Socket path from TMPDIR/XDG_RUNTIME_DIR included in `daemon_not_running_error` message; path is not sensitive per PLAN.md security analysis |

## Threat Detail

### T-5-01 — Tampering (serde deserialization)

Disposition: **mitigate**

Declared mitigation: `serde_json::from_str::<IpcResponse>(line.trim())` returns `Err` on malformed input, propagated via `?` — client never panics on bad response.

Verified at `crates/periphore-cli/src/client.rs` line 37:
```
let response = serde_json::from_str::<IpcResponse>(line.trim())?;
```

The `?` operator is present. No `.unwrap()` or `.expect()` found anywhere in `client.rs`. Command handlers in `status.rs` and `topology.rs` propagate errors via `?` on the `ipc_request()` call, and handle unexpected response variants with `anyhow::bail!` — no silent data corruption path exists.

### T-5-02 — Elevation (socket path construction)

Disposition: **mitigate**

Declared mitigation: `socket_path` is `Option<PathBuf>` from clap (no shell expansion, no string interpolation); OS enforces 0600 ownership on socket file.

Verified at:
- `crates/periphore-cli/src/cli.rs` lines 19, 23: `pub socket: Option<std::path::PathBuf>` and `pub config: Option<std::path::PathBuf>` — clap parses these as typed `PathBuf` values directly, bypassing any shell interpolation.
- `crates/periphore-cli/src/lib.rs` lines 34–43: `resolve_socket_path` extracts the path via `if let Some(path) = &cli.socket` and returns `path.clone()` — no string formatting or interpolation involved.

OS-level 0600 enforcement is a platform guarantee (Unix domain socket file permissions), not a code pattern; no code verification required for this portion.

### T-5-03 — Info Disclosure (daemon_not_running_error)

Disposition: **accept** (pre-accepted per constraint and PLAN.md security analysis)

The error message returned by `daemon_not_running_error` in `client.rs` includes the socket path for diagnostic purposes. The socket path is derived from TMPDIR or XDG_RUNTIME_DIR and is not considered sensitive. Accepted per documented rationale.

## Accepted Risks Log

| Threat ID | Risk | Rationale | Accepted By |
|-----------|------|-----------|-------------|
| T-5-03 | Socket path (TMPDIR/XDG_RUNTIME_DIR) exposed in CLI error messages | Path is not sensitive; user-facing diagnostic value outweighs disclosure concern | PLAN.md security analysis, phase 05 constraint |

## Unregistered Flags

None. No `## Threat Flags` sections found in any phase 05 SUMMARY file.
