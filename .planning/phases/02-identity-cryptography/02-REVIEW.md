---
phase: 02-identity-cryptography
reviewed: 2026-04-22T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - crates/periphored/src/main.rs
  - crates/periphored/Cargo.toml
findings:
  critical: 1
  warning: 1
  info: 3
  total: 5
status: issues_found
---

# Phase 02 (Gap-closure 02-04): Code Review Report

**Reviewed:** 2026-04-22T00:00:00Z
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

This review covers the two files changed by gap-closure plan 02-04: `crates/periphored/src/main.rs` and `crates/periphored/Cargo.toml`. It is comprehensive — all issues from the prior 13-file review that touch these two files are re-evaluated, and new issues introduced by plan-04 changes are assessed.

The plan-04 changes are correct and well-structured. `resolve_identicon` is a clean pure function, its extraction into a free function is the right architectural choice for testability, and the two unit tests correctly exercise both branches of the gate. The `tempfile` dev-dependency is placed correctly and is appropriately scoped.

One critical issue from the prior review remains unfixed: the CPU busy-loop on clean IPC task exit is still present. One new warning-level issue was found: the `send_ok` wildcard arm will silently swallow any future `IpcCommand` variant added without a dedicated dispatch arm, removing all compiler exhaustiveness protection. Three informational items are noted: the ignored `fingerprint` field in `GetIdenticon`/`GetWordPhrase` (carried from prior review, now more relevant since identicon gating is live), the non-uniform `tempfile` version pinning, and the non-`#[cfg(unix)]`-guarded `select!` branches for signal handling.

## Critical Issues

### CR-01: CPU busy-loop when IPC server task exits cleanly

**File:** `crates/periphored/src/main.rs:183-199`

**Issue:** This issue was identified in the prior review and has not been fixed. When the IPC server task completes with `Ok(Ok(()))` (e.g., the Unix socket listener exits without error), the arm at line 185 logs a message and continues the `loop` — it does not `break`. After that point `tasks` is empty. `JoinSet::join_next().await` on an empty `JoinSet` returns `None` immediately without yielding. In Tokio's `select!` the `join_next()` branch therefore wins every poll, the `None => {}` arm at line 196 executes in a tight spin, and the daemon burns 100% of a CPU core indefinitely. The daemon stays alive (signals are still polled), but wastes a full core and provides no forward progress. It will not self-terminate and must be killed externally.

**Fix:**

```rust
// Fix 1 (guard): disable the join_next branch when no tasks remain
result = tasks.join_next(), if !tasks.is_empty() => {
    match result {
        Some(Ok(Ok(()))) => {
            tracing::info!("IPC server task completed");
            break; // Fix 2: clean IPC exit should shut the daemon down, not loop
        }
        Some(Ok(Err(e))) => {
            tracing::error!("IPC server task error: {e}");
            break;
        }
        Some(Err(e)) => {
            tracing::error!("Task panicked: {e}");
            break;
        }
        None => {
            // JoinSet empty — unreachable with the `if !tasks.is_empty()` guard,
            // but Rust requires exhaustiveness.
        }
    }
}
```

Both fixes are needed in tandem: the `if !tasks.is_empty()` precondition prevents the `None` spin, and the `break` in the `Some(Ok(Ok(())))` arm ensures a clean IPC exit triggers graceful daemon shutdown rather than leaving a zombie loop.

## Warnings

### WR-01: `send_ok` wildcard arm silently swallows future `IpcCommand` variants

**File:** `crates/periphored/src/main.rs:241-243`

**Issue:** The `send_ok` function ends with a wildcard `_ => {}` arm. The comment above it (lines 237-240) documents that the explicitly handled commands never reach `send_ok`. This is correct today — all currently handled commands have dedicated arms in the `select!` loop. However, the wildcard permanently removes Rust's exhaustiveness guarantee. When a new `IpcCommand` variant is added in a future phase (e.g., `SetSwitchMode`, `GetConfig`), the compiler will not emit a non-exhaustive match warning or error for `send_ok`. The new command will silently return no response to the client, causing the CLI to hang waiting for a reply that never arrives. This is a latent protocol-level bug waiting to be triggered by normal development.

**Fix:**

Remove the wildcard and list the currently-unreachable commands explicitly with a comment, or add a compile-time note via `#[allow(unreachable_patterns)]` only on the wildcard with a `// KEEP LAST` annotation and a link to the tracking issue. The cleanest approach is to remove the wildcard entirely and only handle what `send_ok` is supposed to handle, letting the compiler enforce completeness:

```rust
fn send_ok(cmd: IpcCommand) {
    match cmd {
        IpcCommand::ListPeers { responder } => {
            let _ = responder.send(IpcResponse::Peers { peers: vec![] });
        }
        IpcCommand::GetTopology { responder } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::AcceptFingerprint { responder, .. } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::RejectFingerprint { responder, .. } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::GetState { responder } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        IpcCommand::GetPendingVerifications { responder } => {
            let _ = responder.send(IpcResponse::Ok);
        }
        // Commands with dedicated select! arms — these never reach send_ok.
        // List them explicitly so the compiler enforces completeness when
        // new IpcCommand variants are added.
        IpcCommand::GetStatus { .. }
        | IpcCommand::InjectInputEvent { .. }
        | IpcCommand::SimulateEdgeCross { .. }
        | IpcCommand::GetIdenticon { .. }
        | IpcCommand::GetWordPhrase { .. }
        | IpcCommand::ReloadConfig { .. } => {
            // Unreachable by construction — each has a dedicated select! arm.
            // If this branch is hit, a routing bug exists in the main loop.
            tracing::error!("send_ok called for a command that should have a dedicated arm — this is a bug");
        }
    }
}
```

## Info

### IN-01: `GetIdenticon` and `GetWordPhrase` ignore the `fingerprint` IPC field (carried from prior review)

**File:** `crates/periphored/src/main.rs:151-163`

**Issue:** `IpcCommand::GetIdenticon { fingerprint, .. }` and `IpcCommand::GetWordPhrase { fingerprint, .. }` both use `..` to discard the `fingerprint` field. Both arms always return the daemon's own identity regardless of what fingerprint the client requested. This issue was present in the prior review; it is re-raised here because the identicon gating change in plan-04 makes the `GetIdenticon` path production-complete for the self-identity case, which increases the likelihood that a CLI client will attempt to use the fingerprint field for peer lookup.

A client passing a peer fingerprint to `GetIdenticon` will silently receive the daemon's own identicon — no error, no indication the field was ignored. This is a correctness gap at the protocol boundary.

**Fix:** Either (a) remove the `fingerprint` field from `IpcCommand::GetIdenticon` and `IpcCommand::GetWordPhrase` until peer lookup is implemented, or (b) return `IpcResponse::Error { message: "peer fingerprint lookup not yet implemented".into() }` when the passed fingerprint does not match the daemon's own `identity.fingerprint_hex()`. Option (a) is cleaner for Phase 2; option (b) gives better client-facing error messages.

### IN-02: `tempfile` dev-dependency not workspace-pinned

**File:** `crates/periphored/Cargo.toml:29`

**Issue:** `tempfile = "3"` is declared directly in `[dev-dependencies]` without going through the workspace. Other crates in this workspace (`periphore-identity`, `periphore-ipc`) also use `tempfile` in their test suites. If each crate pins a different patch version via their own `Cargo.toml`, Cargo may resolve multiple versions of `tempfile` and inflate compile times, or subtle test-helper behavioral differences may emerge across patch versions.

**Fix:** Add `tempfile` to the workspace `[dev-dependencies]` table in the root `Cargo.toml` with a pinned version, then reference it as `tempfile = { workspace = true }` in all crate `Cargo.toml` files that use it. This is consistent with how all other dependencies in this crate are managed.

### IN-03: Signal-handling `select!` branches are not `#[cfg(unix)]`-guarded

**File:** `crates/periphored/src/main.rs:118-127`

**Issue:** The `sigterm` and `sighup` variables are declared under `#[cfg(unix)]` (lines 89-94), but the `select!` branches that reference them (`sigterm.recv()` at line 118 and `sighup.recv()` at line 124) have no corresponding `#[cfg(unix)]` guard. On a non-Unix target these branches would produce compile errors referencing undefined variables. Windows is explicitly out of scope per project constraints, so this cannot cause a real build failure today, but it is a correctness gap for any future cross-platform work and is also a style inconsistency with the declaration guards.

**Fix:** Wrap the signal branches inside the `select!` with `#[cfg(unix)]`:

```rust
loop {
    tokio::select! {
        #[cfg(unix)]
        _ = sigterm.recv() => {
            tracing::info!("SIGTERM received -- shutting down");
            break;
        }

        #[cfg(unix)]
        _ = sighup.recv() => {
            tracing::info!("SIGHUP received -- config reload not yet implemented (Phase 4)");
        }

        cmd = ipc_cmd_rx.recv() => { /* ... */ }

        result = tasks.join_next(), if !tasks.is_empty() => { /* ... */ }
    }
}
```

`tokio::select!` supports per-branch `#[cfg(...)]` attributes since Tokio 1.x.

---

_Reviewed: 2026-04-22T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
